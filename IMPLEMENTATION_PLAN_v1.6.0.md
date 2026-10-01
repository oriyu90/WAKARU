# WAKARU v1.6.0 実装計画書

作成日: 2026-10-01

対象: 1GB 級資料の取り込み・閲覧、AI の対話型図解、資料ビューの付箋

状態: 設計・実装計画。ここに記す変更は未実装、実機性能値は未測定。

## 1. 結論とリリース範囲

v1.6.0 では「1GB のファイルを受け付けた」という状態だけを成功としない。**原本の安全な取り込み、利用できるページの表示、検索可能な範囲、ZIP 往復、再起動後の復元**をそれぞれ判定する。PDF は原本のページ表示を段階取得にする。DOCX・PPTX・表計算・テキストは形式別の段階表示を用意し、元のレイアウトを忠実に再現できない要素がある場合は画面で明示する。壊れたファイル、暗号化されたファイル、展開量が異常なファイルの無条件成功は対象外。

対話型図解は HTML・CSS・SVG・Canvas・JavaScript の自己完結した小さな成果物を共通形式で保存し、ライブ解説と Studio に同じプレビュー部品で表示する。ユーザー提示の React/Tailwind/Three.js 等の表は参考となる機能例であり、ChatGPT 内部実装の確定情報や v1.6.0 の必須依存関係とは扱わない。v1.6.0 の必須範囲は 2D 図解、操作、アニメーション、数式の表示である。3D と任意の npm パッケージの実行は別途評価する。

付箋は**資料そのものに結び付く**。ページ切替、ズーム、ウィンドウのサイズ変更、プロジェクトの閉じ直し、ZIP 書き出しと再読み込み後も、同じ位置と本文を復元する。

## 2. 現行実装の調査結果

| 経路 | 確認した事実 | v1.6.0 への影響 |
| --- | --- | --- |
| 追加 | `sources::add_one` はハッシュを取ってから原本をコピーし、`add_files` はその後に解析ジョブを起動する。1GB の読み取りが少なくとも二巡し、追加コマンドの応答までコピーがかかる。 | コピーとハッシュを同時に行うバックグラウンドジョブ、進捗、キャンセル、空き容量検査が必要。 |
| 解析 | `ingest::run` は `Vec<Unit>` と全体 `document.md` を保持してから DB に書く。PDF は `fs::read` → `extract_text_from_mem_by_pages`、テキストと CSV も全体読み込み。DOCX は主 XML を `String` に展開する。 | ファイルサイズだけでなく、展開後テキスト・ページ数・チャンク数がメモリと DB を押し上げる。形式別の逐次処理が必要。 |
| 資産配信 | `lib.rs::asset_response` は `fs::read` した全バイトを `Response` に載せる。Range / 206 がない。 | 1GB を WebView へ一度に渡す経路を塞ぎ、部分取得を実装する。 |
| PDF / Office 表示 | `FilePreviews.tsx` は 96 MiB を超える PDF・DOCX・PPTX の対話型表示を止める。以下でも `arrayBuffer()`、`.slice(0)`、PDF worker、DOM / Canvas に複製する。PDF のキャンバスは 16M 画素に制限済み。 | 96 MiB を単に引き上げない。PDF は URL / Range、Office は別の段階表示へ切り替える。既存の画素上限は維持する。 |
| 文字・表の表示 | `Preview.tsx` は原本または `document.md` を丸ごと `res.text()` し、CSV は全行を parse してから先頭 2000 行だけ描画する。 | 1GB のテキストを UI に渡さない。ページ API と仮想化が必要。 |
| Studio とライブ解説 | `build_site` は既に HTML/CSS/JS のフォルダを作れるが、Studio では成果物一覧から取り込み、資料ビューへ移動する経路。ライブ解説と Studio の会話は `Markdown` の文字列だけを表示する。 | 会話の中に図解を紐付ける型付き成果物と共通表示部品を追加する。 |
| 既存の iframe | ローカル website と外部ページは `sandbox="allow-scripts"` で表示。`Markdown` は raw HTML を実行しない。 | この分離を維持し、AI のコードを会話 DOM に注入しない。 |
| ZIP | `export.rs` は各ファイルを `read_to_end` して圧縮し、import も各 entry を `read_to_end` して保存する。`project.db` は WAL 使用中の実ファイルを列挙している。import 展開上限は 4 GiB。 | 1GB 原本の ZIP 往復にメモリスパイクが起きる。DB の一貫した snapshot と逐次入出力が必要。 |
| 永続化 | 資料ごとの `project.db` と `viewer_tabs` の locator はあるが、付箋のテーブルはない。source 削除では関連行を cascade する構造。 | `004_notes_visuals.sql` を追加し、資料 ID・ページ ID を使って復元する。 |

主な確認箇所: `src-tauri/src/services/{sources,assets,export,viewer,studio}.rs`、`src-tauri/src/services/ingest/`、`src-tauri/src/lib.rs`、`src/features/viewer/{Preview,FilePreviews,Viewer}.tsx`、`src/features/studio/Studio.tsx`、`src/features/viewer/IllustratorDrawer.tsx`、`src-tauri/migrations/project/001_init.sql`。症状の実ファイル、ファイル形式、macOS の空きメモリ、クラッシュログは今回提示されていないため、どの段階で止まったかの断定はしない。

## 3. 1GB 級資料の実装

### 3.1 状態と取り込みジョブ

1. `source_add_files` は原本パスを検証してジョブ ID と仮の source をすぐ返す。`copying → preview_ready → analyzing → ready / ready_partial / failed` を資料一覧に示す。`preview_ready` なら全文索引がまだなくても資料を開ける。既存 `SourceStatus` の利用箇所を調べ、追加状態は後方互換な任意フィールドとして導入する。
2. `sources/<id>/.incoming` に 1–4 MiB の固定バッファでコピーしながら SHA-256 を計算する。原本の `metadata` を前後で比較して途中変更を検出し、flush / sync 後に原子的 rename。キャンセル・失敗時は一時ファイルと仮行を整理する。重複判定はハッシュ確定後に行い、二重取り込みを残さない。
3. コピー前に原本サイズと project 領域の空き容量を確認し、必要量と不足量を表示する。ファイルサイズだけでは ZIP の展開量を推定できないため、解析には別の上限を置く。Finder/iCloud の未ダウンロード資料や権限エラーは、読み取り失敗と区別する。
4. 解析は `Unit` を一件ずつ DB へ渡す sink に変更し、`document.md` も `BufWriter` へ逐次書く。チャンク生成、言語推定、FTS 登録を小さなバッチ単位にし、全文 `Vec<Unit>` / 巨大 `String` / `collect()` を除く。再解析は別テーブルまたは一時 DB に成果を作り、成功時だけ旧索引と切り替える。中断後も既存の検索結果と付箋は残す。
5. 形式別に展開バイト数、ZIP entry 数、圧縮比、ページ/行数、単位あたりの文字数、処理時間を設定し、超過時は `ready_partial` と未解析範囲を表示する。失敗を「すべて解析済み」と表示しない。暗号化 PDF / OOXML、破損 ZIP は理由付きの失敗とする。

### 3.2 表示方式

| 形式 | v1.6.0 の経路 | 品質と制約 |
| --- | --- | --- |
| PDF | `wakaru-asset` に `HEAD` / 単一 `Range` の `206`、`Content-Range`、`Accept-Ranges`、`Content-Length`、`416` を実装。1 リクエストを上限付きにし `File::seek + take` で読む。PDF.js は `getDocument({url, disableStream:true, disableAutoFetch:true, rangeChunkSize: 256*1024})` でページを必要時に取得する。 | 既存の 16M 画素上限、前の render の取消し、page cleanup を維持。PDF 構造次第で先頭ページ以外の byte range も必要。macOS の WKWebView で Range が届かない場合は `PDFDataRangeTransport` と上限付き Tauri IPC 読み取りへ切り替える。 |
| テキスト / Markdown / JSONL / CSV | `source_read_window(sourceId, offset/line, limit)` で 64–256 KiB のページを返す。UI は仮想リスト、検索ヒットはオフセットへジャンプする。CSV はヘッダと行番号を保持する。 | 巨大 JSON の単一オブジェクトは全構造解析を約束せず、テキスト閲覧と部分索引を明示する。 |
| DOCX / PPTX / XLSX | ZIP entry の総展開量を検査し、XML を逐次読む。DOCX は節・段落単位、PPTX は slide 単位、XLSX は sheet / 行窓で派生表示を生成し、画面では必要な単位だけ取得する。96 MiB 以下の現行レンダラは引き続き利用する。 | 大容量版の派生表示では複雑な配置、埋め込み動画、マクロ、変更履歴を完全再現できない。原本に近い表示が必須な場合、macOS の別プロセス変換器を採用できるか性能・ライセンス・配布サイズを測って判断する。Quick Look のサムネイルだけを全文表示の代わりにしない。 |
| 画像 / 音声 / 動画 | WebView に全体 fetch しない。画像はピクセル数を検査して縮小派生画像を作る。メディアは Range と既存の native element を利用する。 | 元画像の大きさとデコード後メモリを分けて制限する。 |

小さい資産の既存 API は維持し、大きい原本の無制限 GET は拒否して UI に部分取得経路を使わせる。`Access-Control-Allow-Origin: *` と資産パスの解決は、図解 iframe から他資料を読めないように同時に監査する。少なくとも明示的 `sources/<別ID>/...` が要求 URL の source ID を越えて解決されないようにする。

### 3.3 macOS と配布環境の検証

本アプリは macOS では WKWebView の web content process 上で React / PDF.js を動かす。WebKit はそのプロセスが終了する場合があり、ブラウザ内で 1GB を複製しない設計を優先する。macOS 12 という現行最低版を守り、Apple Silicon の実機で開発版と署名済みアプリの双方を測る。PDFKit `PDFDocument(url:)` は URL から開けるが、**1GB で一定メモリに収まることは公式 API だけでは保証されない**ため、必要になった場合の比較実験対象とし、前提にはしない。Windows / Linux は同じ部分取得 API と表示契約で動作させる。

## 4. AI による対話型図解

### 4.1 共有データ契約

`VisualPreview` を文章とは別の型付き成果物にする。最小契約は `{id, schemaVersion, title, html, css, js, data, aspectRatio, sourceRefs, initialState}`。HTML/CSS/JS 合計 256 KiB、state 16 KiB、SVG / Canvas の表示領域と要素数に上限を設ける。リモート URL、CDN、外部 font、`import()`、ネットワーク fetch、フォーム送信は v1.6.0 の図解では不可。画像が必要なら検査済みの data URL を小容量で同梱する。数式は既存 KaTeX または生成 SVG を使えるようにし、host のテーマ値は数値・色だけ渡す。

`visual_previews` を project DB に追加し、生成したコードと出典、作成モデル、作成日時、検証結果を保持する。`message_visuals(message_id, visual_id, ordinal)` で Studio / Live の会話へ、`illustrations.visual_id` で保存済みの最初のライブ解説へ関連付ける。出典は既存の Rust 側解決を使い、図中の文字列を出典として信用しない。古い message / illustration は図解なしでそのまま読める。

### 4.2 生成経路

- **Studio**: 専用ツール `create_visual_preview` を `tool_defs` に加える。`build_site` は引き続き複数ファイルのサイト制作に使い、会話内図解は専用ツールへ誘導する。ツール引数を実行前に型・サイズ検証し、保存後に `visualId` を返す。保存できない結果を完成図解として表示しない。
- **ライブ解説**: 現行 `generate` / `ask` は Markdown のストリームであり Studio のツールループとは独立している。`illustrator_generate_visual(projectId, sourceId, locator, threadId?, instruction)` を追加し、現在の資料範囲と詳細度をもとに専用の JSON 応答を生成・検証する。「図で説明して」という質問と明示的な「図解を作る」操作の両方から呼べるようにする。通常の説明文ストリームへ HTML を混ぜない。
- **表示**: `InteractivePreview` を両画面で使用し、読み込み中、生成失敗、再試行、コピー、拡大、リセットを共通化する。作成済み図解は会話の該当発言直後に表示する。再起動後も DB の ID から再現する。狭い Drawer では縦長、Studio では幅に合わせ、`ResizeObserver` で再描画する。

### 4.3 実行境界

モデル生成コードは親の React DOM に注入しない。専用のプレビュー配信経路を設け、`iframe sandbox="allow-scripts"`（`allow-same-origin` なし）で不透明 origin にする。配信側 CSP は `default-src 'none'`、nonce 付きの同梱 script/style、`connect-src 'none'`、`frame-src 'none'`、`worker-src 'none'`、`object-src 'none'`、`form-action 'none'` 等を基本にする。親アプリの IPC bridge と project asset URL は渡さない。必要な状態通知だけ `postMessage` の schema・サイズ・送信元 window・一回限り token を照合する。iframe 自体の WebKit / Tauri IPC 到達性は署名済み macOS ビルドで攻撃テストする。

JavaScript の無限ループは同じ web content process を止め得る。iframe の隔離だけで CPU / メモリの強制制限ができるとは扱わない。実機試験で親画面まで固まるなら、JS 実行を専用 webview / process に移すか、許可された宣言型図解のみ自動表示して任意 JS は明示起動にする。これをリリース判定項目とする。

## 5. 資料上の付箋

### 5.1 操作と配置

1. 資料の描画面を右クリックすると、その座標に小さな丸いマーカーと空の付箋を一件作る。資料外、ツールバー、ページ送りボタン、テキスト選択の標準メニューでは作らない。マーカーを右クリックするとその付箋を削除し、短時間の「元に戻す」を表示する。マーカーの右クリックは新規作成へ伝播させない。通常の右クリックメニューは対象領域だけ抑止し、macOS の副ボタン・Control+クリックの両方を製品ビルドで確かめる。
2. 資料の上に付箋用の余白を設け、カードの下端だけ資料へ少しはみ出させる。表示中のページ / 節のカードを、追加順に横へずらし、少しずつ縦を下げる階段状に重ねる。他ページの付箋は件数とページ移動から開く。狭い画面では横スクロールと件数表示に切り替え、資料本文やページ操作を覆わない。最前面のカード、ホバー、キーボード focus の重なり順を明確にする。
3. カードまたはマーカーをクリックすると本文の編集欄を開く。編集中のマーカーだけ白い光輪を表示し、編集を終えると戻す。本文は plain text、最大 4,000 文字。入力は debounce 保存し、blur / タブ切替 / プロジェクト離脱時に flush。保存中・失敗の表示と再試行を付ける。丸ぽちと付箋は同じ ID で結ぶ。
4. キーボードでは資料に focus して Shift+F10 / Menu キーで「現在位置に付箋」を開ける。付箋一覧、編集、削除、元に戻すを Tab / Enter / Escape で操作できる。色だけに依存せず、focus ring と `aria-label` を付ける。

### 5.2 座標と永続化

`notes` テーブル案: `id TEXT PRIMARY KEY, source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE, locator TEXT NOT NULL, anchor_kind TEXT NOT NULL, anchor_json TEXT NOT NULL, body TEXT NOT NULL, color TEXT NOT NULL, stack_order INTEGER NOT NULL, created_at TEXT, updated_at TEXT, deleted_at TEXT NULL`。`(source_id, locator)` に索引を張る。IPC は `notes_list`, `notes_create`, `notes_update`, `notes_delete`, `notes_restore`。Rust 側で source の所属、locator、有限な `x/y`、本文長、削除済み状態を検証し、更新は transaction と version / `updated_at` で競合を検出する。

- PDF、スライド、画像: ページまたはスライド番号と**原資料の表示面**に対する `x,y ∈ [0,1]` を保存する。右クリック点は `getBoundingClientRect()` から資料面へ逆変換し、ズーム・DPR・ウィンドウ変更時は再投影する。PDF は current page が描画済みの時だけ設置を受ける。
- DOCX・テキスト・Markdown・表: 可変レイアウトの `%` 位置だけでは再配置に弱い。節 / 段落 / 行 ID と文字オフセット、短い前後文の fingerprint を主 anchor とし、再描画時に解決する。解決できない場合はその節の先頭に「位置の確認が必要」な付箋として出し、消さない。
- 外部 Web ページ / 埋め込み site: iframe 内のスクロール座標を安全に観測できないため、v1.6.0 では reader text / WAKARU 側の資料面へ付ける。サイトの内部ピクセルに貼り付けられるかのような表示はしない。

付箋コンポーネントは `Preview` と各形式レンダラが返す「資料面の矩形・locator・読み込み状態」を使う共通 overlay にする。`viewer_tabs.locator` は閲覧位置のまま維持し、付箋をタブ ID に結び付けない。これによりタブを閉じても付箋は残り、source 削除時だけ cascade する。付箋は閲覧者の私的メモであり、AI の検索・プロンプトには自動投入しない。

## 6. ZIP・再起動・データ移行

`004_notes_visuals.sql` を追加し、`PROJECT_SCHEMA_VERSION` を `1.1.0` とする。旧 project は起動時に forward migration、新規 project は新スキーマで作る。移行前の `project.db` バックアップを作り、失敗時は旧 DB を開ける状態で止める。図解と付箋は project DB に入り、原本・派生表示は project フォルダへ入るため ZIP 対象に含める。manifest に資料数に加え note / visual 件数と検査用 digest を追加する。旧 ZIP は 0 件として import する。

ZIP export は一時ファイルへ書き、`std::io::copy` で各 entry を逐次圧縮する。DB は SQLite online backup または `VACUUM INTO` で一貫した snapshot を採り、その snapshot を入れる。原本のサイズを再確認し、書き出し中の変更・容量不足・中断時には完成 ZIP を残さない。ZIP64 の条件と大きな単一 entry を検証する。import は entry ごとに展開量・件数・パス・symlink・圧縮比・CRC を検証しながら一時 project に逐次書き込み、DB migration / `integrity_check` / 付箋 anchor の参照整合性まで通ってから app DB に登録する。現行 4 GiB の展開上限は 1GB 原本と派生データを含めた実測で見直すが、無制限にはしない。

再起動テストは「閲覧中ページ、ウィンドウの大きさ、倍率、付箋の本文・順序、図解の操作状態、Studio 会話」を個別に確認する。図解の操作状態は安全な JSON だけを最大 16 KiB で保存し、コード実行時の任意オブジェクトを永続化しない。失敗した一時コピー・ZIP・変換成果物は起動時 GC の対象とする。

## 7. 実装順と変更先

| 順 | 作業 | 主な変更先 | 完了条件 |
| --- | --- | --- | --- |
| 0 | 1GiB の合成 PDF / DOCX / PPTX / CSV と壊れた ZIP を作り、現行版の取り込み時間、ピーク RSS、WebKit 終了、ファイルサイズを計測。Range と図解 iframe の macOS 実機 spike。 | `src-tauri/tests/`、計測用 fixture / 手順 | 形式別の失敗段階と採用方式が記録される。 |
| 1 | コピー・ハッシュ・解析を分離し、逐次 sink と部分成功を実装。 | `sources.rs`、`ingest/*`、`jobs/mod.rs`、source IPC / UI | コピー中に UI が応答し、途中失敗後も状態と旧索引が整合する。 |
| 2 | 部分資産配信と PDF.js の URL 読み、テキスト / Office の段階表示。 | `lib.rs`、`assets.rs`、`FilePreviews.tsx`、`Preview.tsx`、新しい read window IPC | 1GiB fixture の指定ページを表示し、全体 `arrayBuffer()` を呼ばない。 |
| 3 | ZIP の streaming と DB snapshot、import の整合性検査。 | `export.rs`、`storage/*` | 1GiB 原本を含む ZIP が往復し、付箋なし旧 ZIP も読める。 |
| 4 | 付箋 migration、IPC、共通 overlay、形式別 anchor。 | `004_notes_visuals.sql`、`domain/viewer.rs`、`commands/viewer.rs`、`services/viewer.rs`、`features/viewer/*` | 右クリック・階段表示・編集・削除・resize・再起動・ZIP 往復が通る。 |
| 5 | 図解の型、Studio tool、Live 生成、隔離した renderer と保存。 | `services/studio.rs`、`services/illustrator.rs`、両 domain、`Studio.tsx`、`IllustratorDrawer.tsx`、新 `InteractivePreview` | 同じ成果物が両画面に表示され、無許可 IPC・ネットワークへ到達しない。 |
| 6 | 日英中 UI、アクセシビリティ、性能と配布監査。 | `src/i18n/*.json`、docs、release / quality 文書 | 下記の受入条件を満たす。 |

上の順番は独立した作業の前提関係を示す。図解と付箋の DB migration は一本にまとめても、UI 作業は別にレビューする。型変更後は `npm run bindings` で生成型を更新する。

## 8. 受入試験と発売停止条件

### 自動試験

- **大容量**: 疎な 1GiB ファイル、実ページを持つ 1GiB PDF、メディア入り DOCX / PPTX、100万行 CSV、壊れた ZIP、暗号化 / 圧縮爆弾を区別する。コピー / 解析 / Range / 再解析の取消しと再起動後の状態を確認。部分配信は `0-0`、末尾、範囲外、複数 Range、HEAD、CORS、別 source ID、symlink を検査する。
- **描画**: WKWebView 実機で先頭・中間・末尾ページ、連続 page めくり、拡大、Retina、画面縮小、メモリ圧迫、OCR を確認する。大容量 Office は内容の欠落箇所を表示し、黙って空白にしない。ピーク RSS、初回表示時間、ページ移動時間を計測し、基準値は作業 0 の実測後に固定する。WebKit の content process 終了、OS による強制終了、アプリの操作不能は失格。
- **図解**: SVG・Canvas・入力スライダー・アニメーション・数式を Live / Studio の両方で生成、保存、再表示する。壊れた JSON / HTML、過大入力、外部 script、fetch、`window.top`、`window.__TAURI_INTERNALS__`、偽 `postMessage` を試し、親 UI・IPC・他資料が操作されないことを確認する。JS 無限ループの回復試験を含む。
- **付箋**: ページごとの同座標、ズーム 0.25–4、最小ウィンドウ 960×640、拡大表示、複数ディスプレイ / Retina、100 件の階段表示、編集中の白い光輪、右クリック削除と undo、入力中の画面遷移、競合更新を確認する。再起動と ZIP export/import 後に本文と anchor が一致する。
- **移行**: 1.5.0 project / ZIP、空 project、資料削除、再解析、失敗した migration、WAL に未 checkpoint の書き込み、ZIP 中断を確認する。

既存の `npm test`、型検査、lint、Rust fmt / clippy / test、日英中 i18n、署名済み macOS アプリ、Windows / Linux の基本閲覧を release gate にする。利用者指定の LAN MLXBar / `Qwen3.8-27B-MLX-4bit` は**図解生成の実モデル受入試験が必要になった実装段階**でのみ接続する。認証値は実行時の OS 資格情報または環境変数から渡し、計画書、fixture、ログ、Git に記録しない。今回の計画作成ではモデル通信試験は行っていない。

発売停止: 1GiB 資料で UI が固まる / WebKit が落ちる、取り込んだ内容を読めないのに成功表示する、ZIP 往復で資料・付箋・図解が消える、ズーム / resize で付箋がずれる、生成コードから Tauri IPC・他資料・ネットワークへ到達できる、図解の JavaScript が親画面を長時間操作不能にする、旧 project が開けなくなる、のいずれか。

## 9. 根拠と技術上の未確定事項

- [PDF.js API](https://mozilla.github.io/pdf.js/api/draft/module-pdfjsLib.html): URL / `PDFDataRangeTransport`、Range、`disableAutoFetch`、`disableStream` の契約。macOS の custom protocol との相性は実測が必要。
- [HTTP Range requests](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Range_requests): `206` / `416` と `Content-Range`。
- [Tauri CSP](https://v2.tauri.app/security/csp/) と [MDN iframe sandbox](https://developer.mozilla.org/en-US/docs/Learn_web_development/Core/Structuring_content/General_embedding_technologies): 生成コードを別 origin へ隔離する根拠。CSP と Tauri IPC の実効性は製品ビルドで確認する。
- [Apple WKWebView content process](https://developer.apple.com/documentation/webkit/wknavigationdelegate/webviewwebcontentprocessdidterminate%28_%3A%29?language=o_8): 描画プロセスの終了は起こり得る。固定の OS メモリ上限値は前提にしない。
- [SQLite Online Backup API](https://www.sqlite.org/backup.html): 使用中の DB から一貫した snapshot を作る方式。
- [Apple PDFKit URL 初期化](https://developer.apple.com/documentation/pdfkit/pdfdocument/init%28url%3A%29-98jte?changes=_7%2C_7): macOS fallback の比較対象。大容量性能は未検証。

未確定: 実際に失敗した資料の形式・構造、WKWebView が custom scheme の Range を PDF.js にどう見せるか、1GiB の複雑な OOXML を元レイアウトのまま表示する現実的な変換器、任意 JS による WebKit 停止の隔離度。作業 0 の結果で詳細方式を確定し、受入条件を緩めずに計画を更新する。
