# WAKARU GUIリメイク計画

日付: 2026-09-07 / v0.0.5

## 目的

既存の画面階層、機能、データフローを維持しながら、アプリ内GUIを
無彩色・会話中心のデスクトップUIへ刷新する。

## 現状分析

- AppShell以下のHome / File Modifier / Search / Settings / Project階層は明確で再利用できる。
- Project以下のViewer / Studio切替、Viewerのソース一覧とプレビュー、Studioの3カラムも維持できる。
- 旧UIは暖色ペーパー、暗い固定トップバー、細いアクセント枠、macOS風セグメントを中心にしており、
  目標とする無彩色面・塗りの選択行・丸い入力面とは視覚言語が異なる。
- トークンと共通コンポーネントが整備されているため、ロジックを変更せずCSS中心に一貫した刷新が可能。

## 実装範囲

1. `tokens.css`: 無彩色ライト/ダーク面、2.25remコントロール、8/12/16px角丸へ更新。
2. AppShell: テーマ同色トップバー、淡いオーバーレイサイドバー、塗りの選択行へ更新。
3. 共通部品: Button / Input / Select / Textarea / Switch / Tabsを丸い面と塗りの主要操作へ統一。
4. 画面: Home、Settings、Search、File Modifier、Project、Viewer、SourceListの余白と面を統一。
5. Studio: 中央会話カラムを維持し、ユーザーバブルと一体型コンポーザーを実装。
6. 文書: `design.md`、UI仕様、README、HANDOFF、DECISIONS、非公開保守メモを同期。

## 非対象

- ルーティング、IPC、Rustバックエンド、データベース、取り込み・検索・AI機能。
- 外部サービスのロゴ、名称、アイコン、その他のブランド資産のコピー。
- バージョン変更、DMG作成、Release公開。

## 受け入れ基準

- 既存の画面階層と主要アクセシブルネームが保持される。
- light/dark/monochrome、80〜150%表示倍率、狭幅/低高レイアウトが既存契約を満たす。
- `typecheck`、`lint`、design rules、hardcoded strings、contrast、i18n、全フロントテスト、
  production buildが通る。
- Home、Sidebar、Settingsを実ブラウザで目視し、面、選択状態、入力、主要ボタンが新しい設計契約と一致する。

## 実施結果

- 実装範囲1〜6を完了。
- バックエンド・IPC・DB・ルーティングの差分なし。
- 検証結果は作業コミットと `docs/HANDOFF.md` に記録する。
