# WAKARU

WAKARUは、手元の資料（PDF・DOCX・PPTX・スプレッドシート・画像・音声・動画・Webページなど）を取り込み、閲覧・検索し、根拠（引用）を確認しながらAIに解説や作業成果を作らせる、ローカルファーストのデスクトップアプリです。

[Studio RIZI / WAKARU](https://studio-rizi.pages.dev/projects/wakaru/) — 紹介サイト / [Releases](https://github.com/oriyu90/WAKARU/releases) — ダウンロード

> このリポジトリでは、配布用アプリ、README、ライセンス、品質情報のみを公開しています。ソースコードは非公開です。

## 主な機能

- PDF、DOCX、PPTX、XLSX／XLS、CSV／TSV、TXT、Markdown、JSON、画像、Webページ、音声・動画の取り込み
- 読み取り専用Viewer、ページ位置を保持するタブ、日本語対応のキーワード検索と、埋め込みAPI設定時の意味検索
- PDF・スライドはページ単位の抽出テキスト表示、画像はズーム表示、音声・動画は再生位置つき文字起こし表示
- 平易な核心、具体例、必要時の短い理解確認を組み合わせた、出典（引用）付きLive Illustrator。資料本文はAIの命令と混ざらないよう分離
- 定型的な前置きや反復を避け、文書作成依頼では成果物をファイルとして保存するStudio
- 接続ごとに選べるOpenAI互換・Anthropic互換API、ローカルWhisper文字起こし、承認制MCPツール、隔離コマンド実行
- 改ざん検証付きプロジェクトZIPの書き出し・読み込み、アーカイブ、完全削除
- 情報欠落が疑われる短すぎるMarkdown整理結果の保存防止
- 日本語、English、简体中文、ライト／ダーク／モノトーン、表示倍率80〜150%。ダークテーマの主要文字は白
- コマンドパレット（Cmd/Ctrl+K）とキーボード操作

画像はプレビューできますが、OCRは含まれません。AI未設定でも、資料の取り込み・閲覧・キーワード検索・TXT保存を利用できます。意味検索には埋め込み対応API、AI解説とStudioの回答生成にはAI接続設定が必要です。

## ダウンロード

[WAKARU v0.0.0](https://github.com/oriyu90/WAKARU/releases/tag/v0.0.0) から `WAKARU_0.0.0_aarch64.dmg` を取得してください。

- **動作条件**: Apple Silicon Mac / macOS 12以降
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
- 開発者サイト: [https://studio-rizi.pages.dev/](https://studio-rizi.pages.dev/)
