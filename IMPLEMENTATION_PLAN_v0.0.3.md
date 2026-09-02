# WAKARU v0.0.3 — LAN API 接続 / レスポンシブUI / 配色コントラスト 修正計画

作成日: 2026-09-02
対象コミット基点: `c7de774`（`main`、作業ツリー clean、v0.0.2 公開済み）
版: `0.0.2` → `0.0.3`（`package.json` / `src-tauri/Cargo.toml` / `src-tauri/tauri.conf.json` の3箇所一致）

## 0. 方針（common rules / WAKARU.md 準拠）

- 既存データ・スキーマを壊さない。v0.0.0 / v0.0.1 / v0.0.2 のプロジェクトを無変更で開けることを回帰確認する。
- APIキー・MCP環境変数・資料本文をログ／テスト出力／リポジトリへ残さない。
- 互換性・既存機能・クラッシュ安全性・メモリ安全性を最優先。`unwrap` 増やさない、poisoned mutex を panic にしない、途中切断を成功扱いしない。
- UIは日本語／English（＋既存の简体中文）で完全対応。`npm run check:i18n` の3言語キー一致を維持。
- デザイン参照: **Studio 画面 = Claude デスクトップアプリ**（中央に測った会話カラム、静かな左右レール、広い作文欄）。**全体の質感 = Hallmark**（暖色ペーパー、単一アクセント、強いタイポグラフィ階層、余白で構造を作る）。`design.md` を唯一の設計契約とし、変更は同ファイルへ反映する（common rules ルール1）。
- ドキュメント更新（README / RELEASE_NOTES / QUALITY_REPORT / studio-rizi 4言語 / common-rules `WAKARU.md`）は**全実装・検証の完了後、最後の工程**として実施する。

---

## 1. 問題1 — ローカルネットワーク上の API キー付き AI へ接続できない

### 症状
`接続テスト` が `3341 ms · 0 models` ＋ トースト `endpoint unreachable`。対象は
`http://192.168.0.165:1234/v1`（OpenAI 互換、KEY 設定済み）。

### 原因分析（コード実測）
1. **`AiClient` の reqwest クライアントがシステム／環境プロキシを継承する。**
   `src-tauri/src/services/ai/client.rs` の `reqwest::Client::builder()` に
   `.no_proxy()` が無い。reqwest 0.12 は既定で `HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY`
   と macOS のシステムプロキシ設定を読む。プロキシ経由で LAN の推論サーバへ到達しようと
   して接続失敗する（VPN・キャプチャプロキシ・企業設定で頻発）。ユーザーが明示設定した
   直接エンドポイントに対しては、プロキシは原則不要かつ有害。
2. **macOS のローカルネットワークプライバシー。**
   バンドル版 Info.plist に `NSLocalNetworkUsageDescription` が無い（Tauri 生成の既定
   plist のまま）。近年の macOS は、バンドルアプリからの `192.168.0.0/16` 等への TCP 接続に
   ローカルネットワーク許可を要求する。使用目的文字列が無いと許可ダイアログが出ず（または
   拒否のまま）、接続がエラーで静かに失敗する。`npm run dev`（Vite）や旧OSでの過去検証が
   通っていたのは、この enforcement 差による環境退行。
3. **診断が握り潰されている。**
   `probe::probe` は `list_models().await.unwrap_or_default()` と `model_ping(...) -> bool`
   で、reqwest の実エラー（connection refused / timeout / proxy / dns / cert）を捨てる。
   ユーザーには常に `endpoint unreachable` としか出ない。`retry()` が connect エラーで
   1s+2s スリープするため所要が約 3.3 秒 = 観測値と一致（=接続そのものの失敗）。
4. **接続テストが retry で遅い。** テスト経路は fail-fast にし、実エラーを即時表示すべき。

### 修正
- **A. プロキシ回避（`AiClient::new`）**
  `reqwest::Client::builder()` に `.no_proxy()` を付ける。対象は AI クライアントのみ
  （資料URL取り込みの `services/ingest/web.rs` の SSRF ガードは無変更）。MCP Streamable
  HTTP クライアント（`services/mcp.rs`）も同方針か、少なくとも private/loopback/`.local`
  宛はプロキシ非経由にする。→ **DECISIONS D-18**。
- **B. ローカルネットワーク使用目的（macOS）**
  `src-tauri/Info.plist` を新規作成し、`NSLocalNetworkUsageDescription`（日本語）を記載。
  併せて `src-tauri/Info.ja.plist` 相当ではなく、`en.lproj` / `ja.lproj` の
  `InfoPlist.strings` で `NSLocalNetworkUsageDescription` をローカライズ。Tauri v2 は
  `src-tauri/Info.plist` を自動マージする。`tauri.conf.json` の macOS 設定に
  `minimumSystemVersion` は現状維持（whisper.cpp 制約で 12.0）。→ **DECISIONS D-19**。
- **C. 診断の可視化（`probe.rs` / `client.rs` / `domain/ai.rs`）**
  `probe::probe` に、到達性チェックの**実際の失敗理由**を `TestResult.note` へ入れる。
  秘密は `sanitise()` を通す。`list_models` とは別に、retry 無し・短タイムアウトの
  `reachability()`（`GET /models` 失敗時のみ 1 トークン `chat` を1回）を用意し、
  `Err` の表示文字列を `note` に格納。`ok=false` のとき UI は `note` をそのまま出す
  （既に `AiSettings.tsx` は `res.note ?? t("ai.unreachable")` を表示する）。
  例: `connect error: プロキシ設定を確認、または macOS のローカルネットワーク許可が必要`。
- **D. UI 補助（`AiSettings.tsx` + i18n）**
  接続テスト失敗時、`note` に加えて定型ヒント（プロキシ／ローカルネットワーク許可／URL 末尾
  `/v1`／`http://` 可）の短い注記を JA・EN・zh で表示。プロファイル編集ダイアログの
  Base URL ヒントに「LAN の `http://IP:PORT/v1` 可」を明記（既に一部あり、文言を精緻化）。
- **E. 回帰テスト（Rust）**
  - `no_proxy` が効いていること: `HTTP_PROXY` を立てた状態で `serve_once` のローカル HTTP に
    到達できる単体テスト。
  - `reachability()` が接続失敗時に非空の `note` を返すこと。
  - 既存の Anthropic / OpenAI ストリームテストは不変で通す。

### 非対象
- 実機 `192.168.0.165:1234` への到達確認はユーザーのマシン／ネットワークが必要（リリース前
  ゲート #1）。macOS 許可ダイアログの受諾もユーザー操作。

---

## 2. 問題2 — UI が端に偏る／画面サイズでボタン等が追随しない

### 症状
`資料を見る` 画面で内容が左に寄り右が広大に空く。ウィンドウ幅を変えてもボタン・レールの
寸法が変わらない（`rem` は表示倍率にのみ追随し、ウィンドウ幅には追随しない）。

### 原因分析
- アプリシェルは `100dvh` 基準で概ね妥当だが、**主要画面の中身に幅の下限確保・センタリング・
  流体スケールが無い**箇所がある。`ProjectPage` は資料一覧だけの状態で右側が空く（Viewer 未
  展開時に「作業カラム」を明示しない）。
- `design.md` の「1つのルート `font-size` が全体を拡縮」は表示倍率設計であり、**ウィンドウ幅**
  に対する流体性は別途必要。現状 `--control-h` 等が固定 `rem` で、狭幅・広幅での密度調整が無い。
- Studio は3カラム grid（`minmax(10rem,14rem) minmax(0,1fr) minmax(12rem,16rem)`）。Claude
  アプリのような**中央会話カラムの最大幅＋左右レールの落ち着いた比率**になっていない。空状態で
  中央が間延びする。
- ブレークポイントが `max-width: 60rem / 42rem / 24rem` 等 rem 基準 = 表示倍率で閾値が動く。
  ウィンドウ実寸に対しては `px` 基準か container query が望ましい。

### 修正（横断・場当たり禁止）
- **A. シェル土台（`base.css` / `AppShell.module.css`）**
  `#app` を `height: 100dvh; display: grid; grid-template-rows: auto 1fr;` に整理。
  `.content` はスクロール所有を明確化し、内側に `--content-pad: clamp(var(--space-sm), 2.5vw, var(--space-xl))` を導入。
- **B. 流体密度トークン（`tokens.css` / `design.md`）**
  ウィンドウ幅連動の派生トークンを追加（`rem` の哲学は維持しつつ `clamp()` で上下限）:
  - `--control-h` は据え置き、`--page-measure: clamp(20rem, 92vw, 76rem)`、
    `--rail-w: clamp(11rem, 18vw, 15rem)`、`--panel-w: clamp(12rem, 22vw, 18rem)` を新設。
  - コンテナ幅に応じて右側レールを畳む順序を `design.md`「150%×960幅で横スクロール禁止」に合わせる。
- **C. 画面別**
  - **Home**: `grid-template-columns: repeat(auto-fill, minmax(min(17rem,100%), 1fr))` は維持、
    ページ全体を `--page-measure` で中央寄せ＋左右 `--content-pad`。
  - **ProjectPage / SourceList**: 一覧のみの状態でも一覧を `--page-measure` に収め中央寄せ。
    Viewer 展開時は既存の2カラム。右の空白は「中央寄せの余白」として意図化。
  - **Studio**: `grid-template-columns: var(--rail-w) minmax(0,1fr) var(--panel-w)` にし、
    会話は `.messages`/`.composer` を `max-inline-size: 48rem; margin-inline: auto`（Claude 風）。
    空状態の中央プレースホルダも同measure。`@container` で `< 68rem` → 右パネル畳み、
    `< 44rem` → 左レール畳み（Drawer 化は現状踏襲）。
  - **Settings / AiSettings / McpSettings / About / ProjectManagement / FileModifier / Search /
    Viewer / IllustratorDrawer**: `max-width` を `min(<既存>, 100%)` に統一、`grid` の固定列を
    `minmax(0, …)` 化、狭高（`max-height`）でヘッダー/フッターが本文を隠さないことを再確認。
- **D. ブレークポイント基準の整理**
  レイアウト分岐は可能な範囲で `@container` + `px`/`ch` に寄せ、表示倍率で閾値がぶれないようにする。
  `check-design-rules.mjs` の `position: fixed` 禁止は維持。
- **E. 実寸再現テスト**
  1440×900 / 1280×720 / 1024×768 / 960×640 / 844×390 / 390×844 / 720×1024 / 1024×480 /
  480×1024、表示倍率 100% と 150%。全主要画面・ダイアログ・ドロワーで右端・下端の欠け、
  操作不能モーダルが無いこと。修正後に同寸法で再検証しスクリーンショットを残す。

---

## 3. 問題3 — 配色のコントラスト不足／黒背景なのに文字が白くない

### 症状
暗いテーマで一次テキスト以外（補助テキスト・境界線・レール見出し）が背景と同化し、
パネル同士の区切りが視認できず「メリハリがない」。primary が白でない印象。

### 原因分析
- `tokens.css` は既にダーク `--color-ink`/`--color-ink-strong`/`--color-topbar-ink` を
  純白 `oklch(100% 0 0)` に修正済みだが、**`design.md` は旧値（91% / 97% / 93%）のまま**で
  契約と実装が不一致（common rules ルール1違反状態）。
- ダーク `--color-rule: oklch(32%)` が `--color-paper: oklch(17%)` 上でほぼ不可視 → パネル境界が
  消える。`--color-rule-strong: oklch(52%)` も弱い。
- ダーク `--color-neutral: oklch(62%)` / `--color-muted: oklch(73%)` の補助テキストが多用され、
  かつ `:root[data-theme="dark"] body { font-weight: 350 }` の極細 + `--fs-2xs`(0.6875rem) で
  可読性が落ちる。
- `--overlay-panel`・`--color-paper-2/3` の階調差が小さく、Studio の左右レール沈み込み
  （`design.md`「Studio はレールを1段沈める」）が視認できない。

### 修正（ライト／ダーク両方、`check:contrast` で数値検証）
- **A. `design.md` と `tokens.css` の再同期**
  ダーク ink 系の純白化を `design.md` にも反映。以降 `tokens.css` を唯一のソースとし、
  `design.md` のトークンブロックを実値へ更新。
- **B. ダークテーマ調整**
  - `--color-rule: oklch(32%) → ~oklch(38–40%)`、`--color-rule-strong: oklch(52% → ~58%)`
    （`rule-strong` vs `paper` は 3:1 以上を維持しつつ体感を強める）。
  - `--color-muted: oklch(73% → ~78%)`（4.5:1 余裕を確保）、`--color-neutral` は
    「アイコン・無効テキスト」用途に限定し本文ラベルには `--color-ink`/`muted` を使う。
  - `body { font-weight: 350 }` を `400`（本文）へ。細字は装飾ラベルのみに限定。
  - `--color-paper-2 / -3` の階調ステップを僅かに広げ、レール・入力欄・沈み面を可視化。
  - Studio レール／workspace の `border-*: var(--color-rule)` を `--color-rule-strong` へ、
    背景を `--color-paper-2`（会話は `--color-paper`）にしてカラム分離を明示。
- **C. ライトテーマ**
  既に AA を満たすが、`--color-rule`（分割線）を僅かに強め、Studio レール沈みを同様に可視化。
  アクセントは `design.md`「ビューポートの3%以下・塗りにしない」を厳守。
- **D. `check-contrast.mjs` のペア拡充**
  `color-neutral on color-paper-2`、`color-muted on color-paper-2`、
  `color-rule-strong on color-paper-2`、`topbar-ink on topbar`（既存）、
  Studio 用 `color-ink on color-paper-2` を追加。ダーク ink 純白の厳格チェックは維持。
- **E. モノクロ（`data-monochrome`）**
  `filter: grayscale(1)` 下でも境界・状態が形状／太さで判別できることを再確認
  （`design.md`「色のみで状態を示さない」）。

---

## 4. 安全性・互換性チェック（全問題共通）

- DB スキーマ変更なし。マイグレーション追加なし。`storage/migrate.rs` の ledger 挙動不変。
- `AiClient` シグネチャ不変（内部の reqwest builder のみ変更）。IPC 契約・`types.gen.ts` の
  差分は `TestResult.note` の意味づけのみ（型は不変）。`npm run bindings` で drift 0。
- パニック経路を増やさない。`no_proxy()` は無条件で失敗しない API。
- ログに URL は出すが、キー・ヘッダー値・資料本文は出さない（既存 `sanitise` / logging 方針）。
- i18n: 追加キーは ja / en / zh-Hans の3ファイルへ同時追加。`check:i18n` 通過。

---

## 5. 検証ゲート（実装後、順に全て）

### 自動
- Frontend: `npm run typecheck` / `npm run lint`（eslint + check-design-rules + check-hardcoded）
  / `npm test`（vitest, axe 含む）/ `npm run check:contrast` / `npm run check:i18n` /
  `npm run build`（tsc + vite）。
- Backend: `cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings &&
  cargo test` / `cargo deny check licenses bans sources`。
- `npm run bindings`（ts-rs drift 0）、`git diff --check`。

### 手動（実寸・実機）
- 第2章 E のビューポート表 × 表示倍率 100% / 150% を実ブラウザ（`npm run dev`）で確認、
  スクリーンショット保存。
- ライト／ダーク／モノクロ各テーマで主要画面の可読性・境界視認を確認。
- **接続テスト（要ユーザー環境）**: `192.168.0.165:1234/v1` + キーで `接続テスト`。
  macOS のローカルネットワーク許可ダイアログを受諾 → `models` 取得と 1 トークン応答、
  役割割り当て、Studio 応答までを実確認。失敗時は `note` に実エラーが出ることを確認。

---

## 6. リリース（全ゲート緑 ＋ 実機接続確認後、最後の工程）

WAKARU.md「バージョンアップ時の更新表」と `RELEASE_DRAFT` の手順に従う。

1. 版を `0.0.3` に統一（`package.json` / lockfile / `src-tauri/Cargo.toml` /
   `src-tauri/tauri.conf.json`）。`docs/DECISIONS.md` に D-18 / D-19 追記。`design.md` 更新。
2. `npm run licenses:generate` で `THIRD_PARTY_LICENSES.md` 再生成。
3. `RELEASE_NOTES.md` / `QUALITY_REPORT.md` を v0.0.3 の変更・検証範囲・既知制約で更新。
4. ビルド: `APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app`。
   DMG は署名済み app + Applications symlink から `hdiutil` で標準作成、`hdiutil verify`、
   マウントして中の app を `codesign --verify --deep --strict`。チェックサムは basename のみ記録。
5. ローカルコミット（`fix:` / `release:` prefix）＋ ローカルタグ `v0.0.3`（remote へ push しない）。
6. 公開リポジトリ `oriyu90/WAKARU`: README / RELEASE_NOTES / QUALITY_REPORT /
   THIRD_PARTY_LICENSES をコピー、4言語 `website/` ページを更新（ソースは置かない）。
7. `oriyu90/studio-rizi`: `website/projects/wakaru/` 4言語 + `website/content.js`
   （releaseDate/Version + UPDATE エントリ）。`npm test && npm run build && npm run validate
   && npm run count-files`。
8. `gh release create v0.0.3 --repo oriyu90/WAKARU --latest --title "WAKARU v0.0.3"
   --notes-file RELEASE_NOTES.md <dmg> <dmg.sha256>`。`v0.0.0`〜`v0.0.2` は履歴として保持。
9. `common-rules-document/WAKARU.md` を最終成果物・コミットハッシュ・本番URLで更新。
10. 公開後、DMG を再取得して名前・バイト数・SHA-256・非draft/非prerelease・Latest を確認。

### ユーザー操作が残る項目
- macOS ローカルネットワーク許可の受諾と実機接続確認（ゲート #1）。
- `docs/09 §8` の GUI スモークテスト最終一巡。
- Google Search Console のサイトマップ再送信（オーナー）。
- Developer ID 署名・Apple 公証（証明書未提供のため ad-hoc のまま）。

---

## 7. 実施順

1. Rust: `no_proxy` + `probe` 診断 + `Info.plist` / `InfoPlist.strings` + 回帰テスト。→ backend ゲート。**（完了）**
2. tokens.css / design.md 再同期 + コントラスト調整 + check-contrast 拡充。→ `check:contrast`。**（完了）**
3. レスポンシブ: base.css / AppShell / 流体トークン → 画面別（Studio 優先で Claude 風）。→ FE ゲート + 実寸検証。**（完了）**
4. i18n 追加キー（3言語）+ AiSettings 失敗ヒント。→ `check:i18n`。**（完了）**
5. 全ゲート再実行。**（完了 — FE/BE 全緑）**

---

## 8. v0.0.3 追加スコープ（2026-09-02、オーナー指示で方針変更）

「SwiftUI で書き換え」はこの環境に該当スキルが無いため、**React/Tauri を維持したまま
macOS ネイティブ風にUIを作り直す**。あわせて**アプリ本体ソースを `oriyu90/WAKARU` へ
履歴ごと公開**（MIT 維持）し、v0.0.3 を正式 Release とする。

> **方針変更の記録**: `docs/最後にやって欲しいことと守って欲しいこと.md` および過去の
> `WAKARU.md` の「ソース非公開」方針を、2026-09-02 にオーナー判断で撤回した。以後
> `oriyu90/WAKARU` はソース公開リポジトリ。→ DECISIONS D-21。

### 8-0. 事前安全確認（完了）

- `.gitignore`: `node_modules` / `src-tauri/target` / `dist` / `*.log` / `.DS_Store` / `coverage`
  / `*.tsbuildinfo` を除外。追跡 266 ファイル、ビルド生成物の混入なし。
- **全履歴 秘密情報スキャン: クリーン**。API キー・トークン・鍵ファイル・`.env` は
  一切なし（`/etc/passwd` はパストラバーサル試験の固定値）。
- 公開時に露出する情報:
  - コミット作者メール `yukiseno0911@gmail.com`（全 30 コミット）。→ 要判断。
  - LAN の IP（`192.168.0.165` 等）が `docs/LIVE_ORNITH_VALIDATION.md` と一部テスト固定値に。
    RFC1918 の私設アドレスで機密ではないが LAN 構成が見える。
  - `docs/最後にやって欲しいことと守って欲しいこと.md`（AI 向け内部指示）。公開前に削除。
- 現公開リポジトリ（ドキュメント専用）を force-push 前にローカルへバックアップ clone。
  v0.0.0〜v0.0.2 の Release/タグは `main` の force-push では消えない。

### 8-1. macOS ネイティブUIリメイク

- `design.md` に「macOS ネイティブ register」を追記（Hallmark の同一性＝暖色ペーパー・
  単一 ember アクセント・8 状態・i18n・WCAG AA・モノクロ・rem は維持）。
- `tokens.css`:
  - `--font-ui` を `-apple-system, "SF Pro Text", "Geist Variable", <CJK>, system-ui` に
    （Geist は bundled fallback のまま＝オフライン invariant I-2 維持）。
  - コントロール寸法を macOS 準拠へ（`--control-h` 2.25rem→約 1.75–1.875rem、tab/toolbar
    高さ、`--radius-md` 6px、フォーカスリング）。
  - サイドバー vibrancy トークン、選択色を macOS 標準の見えに。
- `tauri.conf.json`: ウィンドウを hidden-inset タイトルバー（`titleBarStyle: "Overlay"`、
  可能なら traffic-light 位置指定）。
- `AppShell`: トップバーを unified toolbar 化し、`-webkit-app-region: drag` とトラフィック
  ライトを避ける左インセット。
- コンポーネント CSS: `controls`（NS 風コントロール）、`Tabs`（NSSegmentedControl 風）、
  サイドバー（vibrancy）、リスト行（Finder/Mail 風）、`Dialog`/`Drawer`（macOS シート感）。
- 第2/3章の中央寄せ・コントラスト・Studio Claude レイアウトは維持し、その上に macOS 化。
- 全ゲート再実行 ＋ ビューポート表（1440/1280/1024/960/844/390/720/1024×480/480×1024、
  表示倍率 100%/150%）＋ ライト/ダーク/モノクロ。

### 8-2. 版・文書

- `0.0.3` を `package.json` / `package-lock.json` / `src-tauri/Cargo.toml` /
  `src-tauri/Cargo.lock` / `src-tauri/tauri.conf.json` に統一。
- `npm run licenses:generate`。
- `RELEASE_NOTES.md` / `QUALITY_REPORT.md` / `HANDOFF.md` / `README.md`（ソース公開を反映）
  更新。`docs/最後にやって欲しいことと守って欲しいこと.md` 削除。
- DECISIONS: D-20（macOS ネイティブ register）、D-21（ソース公開への方針変更）。

### 8-3. ビルド・DMG 検証

- `APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app`。
- 署名済み app + Applications symlink から `hdiutil` で標準 DMG、`hdiutil verify`、マウントして
  中の app を `codesign --verify --deep --strict`、起動プローブ、SHA-256、ログ機密スキャン。
- 生成 `Contents/Info.plist` に `NSLocalNetworkUsageDescription` が入っていることを確認
  （D-19 の実効確認）。

### 8-4. ソース公開（不可逆）

- remote `origin` = `https://github.com/oriyu90/WAKARU.git` を追加。
- 変更をコミット（`feat: macOS-native UI remake` → `release: WAKARU v0.0.3`）、ローカルタグ `v0.0.3`。
- `git push --force origin main` ＋ `git push origin v0.0.3`。
- リポジトリ description / homepage 更新（「source stays private」を削除）。
- GitHub 上でソース・LICENSE(MIT)・README 表示・秘密混入なしを確認。

### 8-5. GitHub Release

- `gh release create v0.0.3 --repo oriyu90/WAKARU --latest --title "WAKARU v0.0.3"
  --notes-file RELEASE_NOTES.md <dmg> <dmg.sha256>`。v0.0.0〜v0.0.2 は履歴として保持。
- 資産を再取得して名前・バイト数・SHA-256・Latest を確認。

### 8-6. サイト・保守文書（最後）

- `oriyu90/studio-rizi`: `website/projects/wakaru/` 4言語 + `website/content.js`
  （version / releaseDate / UPDATE）。`npm test && npm run build && npm run validate
  && npm run count-files` → push（Cloudflare 自動デプロイ）。
- `common-rules-document/WAKARU.md`：**方針変更（ソース公開）**、v0.0.3、macOS ネイティブUI、
  新しい公開構成を反映。`RELEASE_DRAFT.md` も。
- メモリ `wakaru-rebuild.md` 更新。
