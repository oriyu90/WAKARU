# WAKARU v0.0.2 完全実装・監査・リリース手順書

作成日: 2026-09-02

## 1. 目的

`資料を見る` で取り込んだ資料を形式に応じて実際に描画し、閲覧・ページ移動・検索・解説へ連携できるようにする。あわせて `docs/DECISIONS.md` D-09〜D-15で意図的に見送っていたコード上の未実装を再設計して完成させ、全品質ゲートと実アプリ試験を通した macOS arm64版を `v0.0.2` の正式Releaseとして公開する。

## 2. 対象範囲

### 2.1 形式別Viewer

| 形式 | v0.0.2の描画方式 | 完了条件 |
|---|---|---|
| PDF | Mozilla PDF.js display layerを遅延読込し、ページをCanvasへ描画 | 全ページ移動、拡大縮小、テキスト層、現在ページ復元、破損PDFの回復可能エラー |
| DOCX | `docx-preview`でOOXMLを隔離DOMへ描画 | ページ、画像、表、ヘッダー／フッターを表示し、抽出テキスト表示へ切替可能 |
| PPTX | MITのブラウザPPTX rendererを遅延読込 | スライド、文字、画像、基本図形を表示し、前後移動と現在位置復元が可能 |
| XLSX/XLS/CSV/TSV | Rustで抽出済みのシート構造を表として描画 | シート切替、行列表示、長大表の上限・スクロール、CSV引用符を正しく処理 |
| Markdown | 既存のサニタイズ済みMarkdown描画 | GFM、数式、コード、原文切替、検索 |
| 画像 | 正規化画像を表示 | fit/100%/拡大縮小、巨大画像でもUIを塞がない |
| 音声／動画 | WebView media要素 + 文字起こし + キーフレーム | 再生、時刻付きセグメント、キーフレーム移動 |
| TXT/JSON/JSONL/code/Web | 既存の安全なテキスト／reader描画を維持 | 折返し、検索、元URL、エラー回復 |

原本の絶対パスはフロントへ渡さず、`wakaru-asset://` のプロジェクト境界検証を必ず通す。Office描画で文書内スクリプト、生HTML、外部参照を実行しない。

## 3. 見送り項目の完了設計

### D-09: PDF／スライド描画

- PDF.js workerをアプリに同梱し、ネットワークCDNへ依存しない。
- PPTXはブラウザ内でArrayBufferを解析し、DOM/SVGへ描画する。
- Rendererは遅延importし、別タブへ移ったらworker・object URL・renderer資源を破棄する。
- 入力サイズ上限、ZIP展開上限、キャンセル、古い描画結果の破棄を実装する。

### D-10: ローカル埋め込み

- `fastembed` と `multilingual-e5-small` を採用する。
- 検索用リモートモデルが未設定ならローカル埋め込みを既定とし、モデルはWAKARUのデータディレクトリへ初回取得する。
- ダウンロード／初期化失敗時は検索全体を失敗させずFTSへ縮退する。
- 文書には `passage:`、検索語には `query:` を付け、既存sqlite-vecのモデル・次元再構築規則を維持する。

### D-11: Vision／OCR／図表解析

- 正規化済み画像ソースをサイズ制限付きdata URLとしてVisionへ渡す。PDFとスライドはViewerで描画し、検索には抽出テキストを用いる。
- Visionロールが設定済みのときだけ解析し、OCR・図表・レイアウト・要点を構造化JSONとして保存する。
- 解析結果は非信頼資料データとして検索・Live Illustratorへ統合し、原文や既存抽出テキストを上書きしない。
- AI未設定、Vision非対応、失敗時は閲覧を阻害せず明示的に未解析状態を示す。

### D-13: Studioストリーミング

- モデルのテキストdelta、tool call開始、tool結果、完了、失敗をタブID付きイベントとして送る。
- フロントは仮メッセージを逐次更新し、キャンセル後の遅延deltaを無視する。
- DBへは確定メッセージのみ保存し、承認待ち・10反復上限・再開処理を維持する。

### D-14: MCP Streamable HTTP

- `rmcp 3.0.1` のStreamable HTTP clientをrustls構成で有効化する。
- `https://` を既定とし、localhost／プライベートLANに限り `http://` を許可する。
- URL資格情報・fragmentを拒否し、認証値はキーチェーン参照で保存する。
- 接続、discover/init、一覧、call、closeの既存timeoutと秘密値マスキングを共通適用する。

### D-15: Silero VAD／動画キーフレーム

- `silero-vad-crs` の埋め込みモデルを使用し、ランタイムDLやONNX共有ライブラリを不要にする。
- 音声を16kHz monoへ変換した後にSilero確率から発話区間を作り、Whisperへ渡す。
- 動画はWebViewで再生可能な範囲をCanvasへサンプリングし、上限8枚の実行時サムネイルとして表示する。
- 未対応コーデックは偽装せず、再生／抽出できない理由を表示する。

## 4. セキュリティ・クラッシュ安全性

1. PDF/OOXML/画像は圧縮サイズ、展開サイズ、ページ数、画素数を制限する。
2. 文書由来HTMLを `dangerouslySetInnerHTML` で無検証挿入しない。Renderer出力は隔離コンテナに限定し、外部URLを無効化する。
3. 解析・描画・AI・MCPにはtimeoutとキャンセルを設ける。
4. APIキー、MCP認証値、資料本文、画像base64をログへ記録しない。
5. DB更新はトランザクション化し、部分解析を既存のreadyデータへ上書きしない。
6. 旧v0.0.0/v0.0.1プロジェクトを無変更で開けることを回帰試験する。

## 5. 実装順序

1. 版番号を0.0.2へ更新し、D-17で過去の見送り判断を覆す。
2. SourceDetailとasset APIを拡張し、原本を安全なURL／ArrayBufferで取得可能にする。
3. PDF、DOCX、PPTX、XLSX/XLS、既存形式のViewerを形式別に実装する。
4. Silero VADと動画キーフレーム保存を実装する。
5. ローカル埋め込みとFTS縮退を実装する。
6. Vision解析の保存・再利用・検索／解説統合を実装する。
7. Studioストリーミングとキャンセルを実装する。
8. MCP Streamable HTTPと認証管理を実装する。
9. 仕様、受け入れ基準、引継ぎ、公開文書を更新する。

## 6. 検証

### 自動

- Frontend: typecheck、lint、unit、axe、contrast、i18n、production build。
- Backend: fmt、clippy（warnings deny）、全test、binding drift、cargo-deny。
- 形式別fixture: PDF、DOCX、PPTX、XLSX、CSV、MD、PNG/JPEG、MP4、破損／巨大／ZIP bomb境界。
- MCP: 公式stdio serverとローカルStreamable HTTP fixture。
- AI: ローカル埋め込み、Vision mock、Studio delta/tool/cancel順序。

### 実アプリ

- `資料を見る` から全fixtureを開き、描画、ページ／スライド／シート移動、再起動復元を確認する。
- 240×320、320×568、568×320、960×640、1280×840、1600×500、表示倍率150%で横欠けを確認する。
- 破損資料、AI未設定、ネットワーク切断、MCP切断、描画キャンセルで白画面・停止がないことを確認する。
- v0.0.1データの起動移行、ログの機密スキャン、DMG内アプリ起動を確認する。

## 7. リリース

1. `package.json`、lockfile、Cargo、Tauriの版を0.0.2へ一致させる。
2. 第三者ライセンスを再生成する。
3. ad-hoc署名のmacOS arm64 app/DMGを作る。
4. DMG、内包署名、起動、SHA-256、GitHub再ダウンロードを検証する。
5. `oriyu90/WAKARU`へソースを含めず、DMG・checksum・README・Release Notes・Quality Reportだけを公開する。
6. v0.0.2を正式ReleaseかつLatestにする。
7. Studio RIZI 4言語ページと非公開common rules文書を更新する。

## 8. 外部条件として残るもの

- Developer ID署名／Apple公証は証明書が提供されない限り実施できない。ad-hoc署名であることを明記する。
- Windows/Linux実機認証は該当OSホストなしでは完了扱いにしない。macOS版のコード未実装とは分離する。
- DRM、暗号化Office/PDF、WebViewが再生できない特許／旧式コーデックは、内容を推測せず明示エラーにする。

既知のコード上の見送りはD-17の範囲で解消する。テキスト層のないPDFの自動OCR索引化、暗号化文書、未対応コーデックは明示した制約として残す。
