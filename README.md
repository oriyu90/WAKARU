# WAKARU

WAKARUは、手元の資料（PDF・DOCX・PPTX・スプレッドシート・画像・音声・動画・Webページなど）を取り込み、閲覧・検索し、根拠（引用）を確認しながらAIに解説や作業成果を作らせる、ローカルファーストのデスクトップアプリです。

[wakaru.pages.dev](https://wakaru.pages.dev/) — 紹介サイト / [Releases](https://github.com/oriyu90/WAKARU/releases) — ダウンロード

> このリポジトリでは、アプリ本体（README・ライセンス・紹介サイト）のみを公開しています。ソースコードは非公開です。

## 主な機能

- PDF、DOCX、PPTX、XLSX／XLS、CSV／TSV、TXT、Markdown、JSON、画像、Webページ、音声・動画の取り込み
- 読み取り専用Viewer、ページ位置を保持するタブ、日本語対応のキーワード／意味検索
- **PDFはアプリ自身がページを描画**（macOS）。OS標準ビューアのコントロールドックが資料の上に出ません
- **資料の見え方**：白黒反転、露出低減、シャープネス、モノトーン
- 出典（引用）付きLive Illustratorと、プロジェクトの作業領域へ成果物を作るStudio
- OpenAI互換AI接続、ローカルWhisper文字起こし、承認制MCPツール、隔離コマンド実行
- 改ざん検証付きプロジェクトZIPの書き出し・読み込み、アーカイブ、完全削除
- 日本語、English、简体中文、ライト／ダーク／モノトーン、表示倍率80〜150%
- コマンドパレット（Cmd/Ctrl+K）とキーボード操作

画像はプレビューできますが、OCRは含まれません。AI未設定でも、資料の取り込み・閲覧・キーワード検索・TXT保存を利用できます。意味検索、AI解説、Studioの回答生成には、それぞれローカル埋め込みモデルまたはAI接続設定が必要です。

## ダウンロード

[Releases](https://github.com/oriyu90/WAKARU/releases/latest) から最新版のDMGを取得してください。

- **動作条件**: Apple Silicon Mac / macOS 11以降
- Windows／Linux向けの実行ファイルは、実機での検証が完了していないため現時点では配布していません
- Developer ID署名とApple公証は未実施です。初回起動時にmacOSの確認が表示される場合があります。Finderで`WAKARU.app`を右クリック→「開く」を選んでください

配布物のSHA-256チェックサムは各Releaseに添付しています。

## プライバシーと通信

テレメトリや自動的な資料送信はありません。通信が発生するのは、ユーザーが実行した次の操作だけです。

- 登録したAI／MCPサーバーへの接続
- URL資料の取り込み
- 埋め込みモデル／Whisperモデルのダウンロード

APIキーはOSの資格情報ストアに保存され、アプリのデータベースや設定ファイルには残りません。資料本文、プロンプト、APIキーはアプリのログへ記録されません。プロジェクトの原本ファイルはアプリが書き換え・削除しません。

## 品質と既知の制約

検証内容は[QUALITY_REPORT.md](QUALITY_REPORT.md)、バージョンごとの変更点は[RELEASE_NOTES.md](RELEASE_NOTES.md)を参照してください。既知の制約（画像OCR未対応、Windows／Linuxは配布対象外、など）もそちらに記載しています。

## ライセンス

MIT License — Copyright (c) 2026 Yuki Orita（折田悠希）。依存ソフトウェアの通知は[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md)にあります。

## コミュニティ

- Discord（不具合報告・お知らせ）: [https://discord.gg/x7KXhNTD8M](https://discord.gg/x7KXhNTD8M)
- X: [https://x.com/InovateofRIZI](https://x.com/InovateofRIZI)
- 開発者サイト: [https://oriyu90.github.io/official/](https://oriyu90.github.io/official/)
