# WAKARU v0.0.4 — スキャンOCR・裏テキスト・Studio 文書生成ツール 手順書

作成日: 2026-09-03
基点: `19f3644`（`main`、v0.0.3 公開後）
版: `0.0.3` → `0.0.4`（`package.json` / `package-lock.json` / `src-tauri/Cargo.toml` /
`src-tauri/Cargo.lock` / `src-tauri/tauri.conf.json` の5箇所一致）

## 0. 決定事項（オーナー回答 2026-09-03）

| 論点 | 決定 |
|---|---|
| Private 化 | **実施済み**（`oriyu90/WAKARU` は PRIVATE）。**v0.0.4 も Private のまま出す** — Release 資産はコラボレーターのみ取得可。studio-rizi のダウンロード導線は当面 404 のまま（サイト側は「準備中」表記へ差し替える案内を出す）。 |
| PDF ラスタ化 | **フロント（PDF.js）に描かせる** を第一候補。困難なら `pdfium-render` 同梱に切替。 |
| CJK フォント同梱 | **許容**（サンドイッチ PDF・Studio PDF を CJK 対応）。Noto Sans SC/CJK をサブセット埋め込み。 |
| Studio 文書機能 | **組み込みツール `build_document`**（`write_file` と並ぶ）。**MD + DOCX + PDF** の3形式。 |

## 1. 目的（3点）

1. 取り込んだ PDF に**テキスト層が無い（画像化された）ページ**がある場合、OCR して
   検索可能にし、元 PDF の座標に整合した**不可視テキスト層付き PDF（`searchable.pdf`）**を生成する。
2. 取り込んだ**画像**に文字がある場合、OCR して見つかった文字列を**裏 txt（サイドカー）**として
   保存し、検索対象に加える。
3. 性能の低い AI でも Studio タブで Docs / PDF を適切に生成できるよう、**コンテキストを食わない
   組み込みツール `build_document`**（＋短い文書スキル・プロンプト）を追加する。

## 2. アーキテクチャ

### 2.1 OCR 基盤 — `src-tauri/src/services/ocr.rs`（新規）

- 依存: `ocrs` + `rten`（**純 Rust**、C 依存なし、MIT/Apache）。モデル
  `text-detection.rten` / `text-recognition.rten` は初回に
  `<data_dir>/models/ocr/` へダウンロード（`services/ai/mod.rs` の `embed_local` と同じ
  `spawn_blocking` + `OnceLock<Mutex<OcrEngine>>` パターン。SHA-256 記録、失敗時は縮退＝
  OCR なしでプレースホルダ維持、**panic しない**）。
- API: `ocr_image_bytes(png: &[u8], min_conf: f32) -> AppResult<OcrPage>`
  - `OcrPage { lines: Vec<OcrLine>, width_px: u32, height_px: u32 }`
  - `OcrLine { text: String, bbox_frac: [f32;4] /* x,y,w,h 0..1 */, words: Vec<OcrWord> }`
  - 座標はページ比率（解像度非依存）で保存する。
- 上限: 入力 30 MP、行数、実行時間（タイムアウト）。プロセス全体ロック（whisper と同様）。
- `ts-rs` 型は最小限（OCR 状態のみ）。

### 2.2 画像 OCR（取り込み）

- `services/ingest/image.rs` 後段、または後続ジョブ: 正規化 PNG（`derived/<sid>/pages/0001.png`）へ
  `ocr::ocr_image_bytes` を実行（設定 `ocr.enabled` 既定 on）。
- 保存物:
  - `documents` に `kind = "ocr"` の Unit（結合テキスト）→ chunk / FTS / 埋め込みへ流し**検索可能化**。
  - `derived/<sid>/ocr.txt`（結合プレーンテキスト＝「裏 txt」）。
  - `derived/<sid>/ocr.json`（行＋語ボックス、比率座標）→ Viewer の透明レイヤー用。
- Locator: 行ごとに既存の `{ t: "region", bbox: [...] }` を再利用。
- `source_detail` に `ocrTextUrl` / `ocrJsonUrl`（`wakaru-asset://`）を追加。

### 2.3 スキャン PDF OCR（フロント補助）

- `services/ingest/pdf.rs`: テキスト抽出が空のページを検出したら、`sources.ocr_status`
  を `pending` にする（マイグレーション 2.6）。プレースホルダ Unit は当面維持。
- フロント（Viewer）: `source.ocrStatus === "pending"` の PDF を開いたとき、または
  「テキストを認識」ボタン押下時に:
  1. 既に読み込んだ PDF.js ドキュメントで、テキスト層の無い各ページを ~200 DPI の canvas に描画。
  2. `canvas.toBlob('image/png')` → PNG → `ocr_page({ projectId, sourceId, page, pngBase64,
     pageWidthPt, pageHeightPt })` IPC。順次（1ページずつ）実行し、DPI を上限で抑える。
  3. Rust: `ocr::ocr_image_bytes` → 保存: `documents`(`kind="ocr"`, page locator) ＋
     `derived/<sid>/ocr/pNNNN.json`（比率座標）。`ocr://progress` イベント発火。
  4. 全ページ完了 → `ocr_status = "done"` → `searchable.pdf` 生成（2.4） → `global_index` 再構築。
- Viewer 透明レイヤー: OCR 済みページの PDF.js canvas 上に、行ごとの `<span>`（透明・
  比率座標→px 変換）を重ねる → 選択・Cmd+F 可能（＝UX 上の「裏 PDF」）。

### 2.4 サンドイッチ PDF — `src-tauri/src/services/pdf_sandwich.rs`（新規）

- 入力: 元 PDF パス、ページごとのラスタ PNG、ページごとの OCR JSON。
- 出力: `derived/<sid>/searchable.pdf`（ページ画像 full-bleed ＋ 不可視テキスト層）。
- 実装: `printpdf` 0.12.7（`ops.rs` に `SetTextRenderingMode`、`font.rs`/`cmap.rs` に
  フォント埋め込み＋ToUnicode あり）で新規 PDF を組み立てる。
  - 各ページ: PNG を XObject で全面配置 → 各語について
    `SaveGraphicsState → SetTextRenderingMode(Invisible) → 埋め込み CJK フォント選択 →
    SetTextCursor(x,y ページ pt) → SetFontSize(≈bbox 高) → WriteText(word) → RestoreGraphicsState`。
  - フォント: `src-tauri/assets/fonts/NotoSansSC-Regular.otf`（または `.ttf`）を同梱、
    `printpdf` にサブセット埋め込みさせる。Latin も同フォントで可。
  - **spike を最初に行う**: printpdf の不可視 CJK テキスト＋サブセット＋ToUnicode が
    実用に足るか検証。不足なら `lopdf`（MIT）＋ `allsorts`（Apache-2.0）で
    Type0/Identity-H + CIDFontType2 + ToUnicode を手組みへ切替（判断を DECISIONS へ）。
- 提供: Viewer ツールバー／ソース行に「検索可能 PDF をダウンロード」（ネイティブ保存ダイアログ）。
  `source_detail` に `searchablePdfUrl`。テキスト層完備の PDF には生成しない。

### 2.5 `build_document` Studio ツール — `services/studio.rs`

- `write_file` と並ぶ組み込みツール。スキーマ:
  ```jsonc
  {
    "name": "build_document",
    "parameters": {
      "type": "object",
      "properties": {
        "path":   { "type": "string" },   // ワークスペース相対、例 report.docx
        "format": { "enum": ["md", "docx", "pdf"] },
        "title":  { "type": "string" },
        "toc":    { "type": "boolean" },
        "sections": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "level":   { "type": "integer", "minimum": 1, "maximum": 4 },
              "heading": { "type": "string" },
              "body":    { "type": "string" }  // Markdown 部分集合（段落/箇条書き/番号/表/コード/太字/斜体）
            },
            "required": ["body"]
          }
        }
      },
      "required": ["path", "format", "title", "sections"]
    }
  }
  ```
- Rust レンダリング（決定論的）:
  - **md**: `# title` ＋（`toc` なら目次）＋ 各 section を `##`〜`#####`（level）＋ body。
  - **docx**: `docx-rs`（MIT）。Title / Heading1–4 スタイル、body の Markdown 部分集合を
    段落・runs へ。CJK は読み手のフォント任せ（埋め込み不要）。
  - **pdf**: 2.4 のフォント基盤を再利用。Markdown 部分集合を `printpdf` ページへ流し込み、
    見出し・改ページ・ページ番号・（`toc` なら）目次。
  - Markdown 部分集合パーサは**小さく保ち**、未対応記法はエスケープして素通しする（誤描画禁止）。
- サンドボックス／承認は `write_file` と同一（`resolve_in_sandbox` パスガード、新規は
  `auto_allow_new_file_writes`、上書きは常に確認）。返り値 `{ path, bytes, format }`。
- **文書スキル・プロンプト**: `prompts/studio.{en,ja,zh-Hans}.md` に 8〜12 行追記
  （「.docx/.pdf/.md を求められたら `build_document` を使う。本文は書かず `sections` に
  見出しと Markdown 本文だけ渡す。整形・改ページ・目次はツールが行う」）。外部 Skill 非同梱・
  Skill 名も出さない（D-16 遵守）。

### 2.6 マイグレーション・互換

- `project.db`: `migrations/project/003_ocr.sql` = `ALTER TABLE sources ADD COLUMN
  ocr_status TEXT;`（nullable）。`storage/mod.rs` の `PROJECT_MIGRATIONS` に登録、
  `PROJECT_SCHEMA_VERSION` を `1.0.0` → `1.1.0`。レガシー検出ブロックに
  「`sources.ocr_status` 列が既にあれば `003_ocr` を適用済みマーク」を追加。
- `version_verdict`: v0.0.3 の `1.0.0` を v0.0.4 が開く → `Migrate`（`003_ocr` 実行）。
  v0.0.4 の `1.1.0` を v0.0.3 が開く → `WarnOpen`（`ocr_status` 列は保持、往復で失われない）。
- `app.db`: 設定追加は `services/settings.rs` の deep-merge で吸収（マイグレーション不要）。
- v0.0.0〜v0.0.3 プロジェクトは無変更で開ける（新列 nullable、`null` は「未評価」）。

### 2.7 設定 + i18n

- `domain/settings.rs` / `services/settings.rs`: `ocr.enabled`（既定 on）、
  `ocr.autoRunOnOpen`（既定 on）。バックエンド `Settings`（表示系ではないので localStorage 不可）。
- i18n: OCR 状態・アクション（「テキストを認識」「検索可能 PDF をダウンロード」「認識中 N/M」
  「文字なし」等）を ja / en / zh-Hans の3ファイルへ同時追加。`check:i18n` 通過。
- 設定画面「文字起こし」節の近くに「文字認識(OCR)」節、または「一般」に統合。

## 3. 新規依存（すべて非 GPL、`cargo deny` 通過想定）

| クレート/資産 | ライセンス | 用途 |
|---|---|---|
| `ocrs` | MIT | OCR エンジン |
| `rten`, `rten-imageproc`, `rten-tensor` | MIT | ocrs のテンソル/前処理 |
| `docx-rs` | MIT | DOCX 生成 |
| `NotoSansSC-Regular`（フォントファイル） | SIL OFL 1.1 | サンドイッチ/Studio PDF の CJK。`THIRD_PARTY_LICENSES` と NOTICE に明記 |
| （必要時）`lopdf` | MIT | printpdf のサンドイッチ経路が不足の場合の代替 |
| （必要時）`allsorts` | Apache-2.0 | フォントサブセット（printpdf 経由で足りなければ） |

`ocrs`/`rten` のモデルはバイナリに含めず初回 DL（fastembed / whisper と同方針）。
アプリ/DMG は増える（22MB → 推定 35〜45MB）。`RELEASE_NOTES` に明記。

## 4. 実装順

1. **依存追加 + `services/ocr.rs`**（エンジン・モデル DL・縮退・上限）＋ 純ヘルパの単体テスト。→ BE ゲート。
2. **画像 OCR**（`ingest/image.rs` 後段、`documents`/FTS/埋め込み、`ocr.txt`/`ocr.json`）＋ 統合テスト。
3. **`003_ocr.sql` + `ocr_status`**（`storage/mod.rs`、レガシー検出、`PROJECT_SCHEMA_VERSION` 1.1.0）＋ 往復テスト。
4. **`ocr_page` コマンド + フロント**（PDF.js ラスタ、透明レイヤー、`ocr://progress` UI、「テキストを認識」）。
5. **CJK フォント同梱 + `services/pdf_sandwich.rs`**（printpdf spike → 必要なら lopdf）＋ 生成 PDF を lopdf で読み戻す検証テスト。
6. **`build_document` ツール + `docx-rs` + PDF + 文書スキルプロンプト**（3言語）＋ 統合テスト（md/docx/pdf 生成、docx を unzip して `word/document.xml` 検査、パスガード、上書き確認）。
7. **設定 + i18n**（3言語）。
8. 全ゲート（FE typecheck/lint/test/contrast/i18n/build、BE fmt/clippy -D warnings/test/deny、bindings drift 0）→ **設計精査＋デバッグ**（問題をまとめて対処）。
9. 版 `0.0.4` 統一、`npm run licenses:generate`、`RELEASE_NOTES`/`QUALITY_REPORT`/`README`/
   `HANDOFF`/`DECISIONS`（D-22 OCR、D-23 build_document、D-24 CJK フォント同梱、D-25 サンドイッチ実装方式）更新。
10. `APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app`
    → 署名済み app から `hdiutil` で DMG → `hdiutil verify` → マウント内署名 → 起動プローブ →
    ログ機密スキャン → SHA-256（basename）。
11. コミット（`feat:` / `release:`）＋ ローカルタグ `v0.0.4` ＋ `git push origin main` ＋ `git push origin v0.0.4`。
12. `gh release create v0.0.4 --repo oriyu90/WAKARU --latest --notes-file RELEASE_NOTES.md
    <dmg> <dmg.sha256>`（**Private リポジトリのため資産はコラボレーター限定**）。
13. studio-rizi: `website/projects/wakaru/` 4言語 + `content.js`（version/date/UPDATE）。
    **ダウンロード導線は「準備中（招待制）」等へ差し替え**（404 ボタンを出さない）。
    `npm test && npm run build && npm run validate && npm run count-files` → push。
14. `common-rules-document/WAKARU.md`（v0.0.4・OCR・build_document・Private 方針）更新。メモリ更新。

## 5. リスク

1. `printpdf` 0.12 の不可視 CJK テキスト＋サブセット＋ToUnicode — 5 で spike、不足なら lopdf。
2. `ocrs` モデル DL 容量・初回時間・進捗 UX（whisper と同様に `ocr://download` を出す）。
3. PDF.js canvas→PNG→IPC のペイロード（200 DPI A4 ≈ 1〜3 MB/ページ）。DPI 上限・逐次処理で抑制。
4. アプリ容量増（CJK フォント ~10MB + ocrs/rten）。`RELEASE_NOTES` に明記。
5. `docx-rs` の Markdown 部分集合パーサ — 小さく保ち、未対応はエスケープ素通し。
6. Private リポジトリでの公開導線の穴（サイトの 404）— 13 で「準備中」表記へ。

## 6. 完了条件

- スキャン PDF（テキスト層なしページを含む）を取り込み→開くと OCR が走り、
  ページ本文が全文検索に載り、Viewer でテキスト選択・Cmd+F ができ、`searchable.pdf` を保存できる。
- 文字のある画像を取り込むと OCR テキストが検索に載り、`ocr.txt` サイドカーができる。
- Studio で「〜のレポートを Word（または PDF）で作って」に対し、弱いモデルでも
  `build_document` 経由で体裁の整った .docx / .pdf / .md がワークスペースに生成される。
- v0.0.0〜v0.0.3 のプロジェクトが無変更で開け、往復（export/import）で `ocr_status` と OCR 派生物が保たれる。
- 全自動ゲート緑、新規の重大・高優先度不具合なし。
- v0.0.4 を Private リポジトリの正式 Release / Latest として公開。
