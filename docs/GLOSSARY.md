# 用語集（3言語対訳）

UIに出す語の訳をここで固定する。**新しい概念語をUIに出すときは、必ずここに3言語分を追記してから実装する。**
機械翻訳をそのまま入れないこと（[`07_設定_i18n_配布.md`](07_設定_i18n_配布.md) §2.5）。

## 中核概念

| 概念 | en | ja | zh-Hans | 備考 |
|---|---|---|---|---|
| Project | Project | プロジェクト | 项目 | |
| Source | Source | ソース | 资料 | 取り込んだ原本ファイル |
| Live Illustrator | Live Illustrator | ライブ解説 | 实时讲解 | 機能名。英語は固有名として維持 |
| Studio | Studio | Studio | 工作台 | 日本語は英語のまま（機能名として定着させる） |
| File Modifier | File Modifier | ファイル変換 | 文件转换 | |
| Workspace | Workspace | 作業領域 | 工作区 | Studioのサンドボックス |
| Artifact | Artifact | 生成ファイル | 生成文件 | 「アーティファクト」は避ける（一般語でない） |
| Citation | Citation | 引用 | 引用 | |
| Thread | Conversation | 会話 | 对话 | UIでは "Thread" を使わない |
| Chunk | — | — | — | **内部語。UIに出さない** |
| Document | — | — | — | **内部語。UIに出さない**（ページ／スライド／区間 と具体語で言う） |
| Locator | — | — | — | **内部語。UIに出さない** |

## 操作

| 概念 | en | ja | zh-Hans |
|---|---|---|---|
| Add source | Add | 追加 | 添加 |
| Analyze / Re-analyze | Re-analyze | 再解析 | 重新解析 |
| Import to Studio | Send to Studio | Studioへ送る | 发送到工作台 |
| Add to sources | Add to sources | ソースに追加 | 添加到资料 |
| Download | Download | ダウンロード | 下载 |
| Export | Export | エクスポート | 导出 |
| Import | Import | インポート | 导入 |
| Archive | Archive | アーカイブ | 归档 |
| Unarchive | Restore | 復元 | 恢复 |
| Delete | Delete | 削除 | 删除 |
| Regenerate | Regenerate | 再生成 | 重新生成 |
| Stop | Stop | 停止 | 停止 |
| Approve / Deny (tool) | Allow / Deny | 許可 / 拒否 | 允许 / 拒绝 |
| Always allow | Always allow | 常に許可 | 始终允许 |

## 状態

| 概念 | en | ja | zh-Hans |
|---|---|---|---|
| queued | Queued | 待機中 | 等待中 |
| analyzing | Analyzing | 解析中 | 解析中 |
| ready | Ready | 完了 | 完成 |
| ready_partial | Partly ready | 一部のみ完了 | 部分完成 |
| failed | Failed | 失敗 | 失败 |
| Read-only | Read-only | 閲覧専用 | 只读 |
| Cached | Saved explanation | 保存された解説 | 已保存的讲解 |

## 設定

| 概念 | en | ja | zh-Hans |
|---|---|---|---|
| Appearance | Appearance | 表示 | 显示 |
| Theme | Theme | 表示モード | 显示模式 |
| Light / Dark / System | Light / Dark / Follow system | ライト / ダーク / システムに従う | 浅色 / 深色 / 跟随系统 |
| Monochrome | Monochrome | モノトーン | 单色 |
| Display size | Display size | 表示サイズ | 显示大小 |
| UI language | Interface language | 表示言語 | 界面语言 |
| AI response language | AI response language | AIの回答言語 | AI 回复语言 |
| Transcription | Transcription | 文字起こし | 转录 |
| Scope: page / source / project | This page / This source / Whole project | このページ / このソース / プロジェクト全体 | 本页 / 本资料 / 整个项目 |
| Detail level | Simple / Standard / Detailed | かんたん / 標準 / くわしい | 简明 / 标准 / 详细 |

## AI関連

| 概念 | en | ja | zh-Hans |
|---|---|---|---|
| Profile (endpoint) | Connection | 接続先 | 连接 |
| Base URL | Base URL | Base URL | Base URL |
| Test connection | Test connection | 接続テスト | 测试连接 |
| Role: chat | Chat model | チャット用モデル | 对话模型 |
| Role: vision | Image understanding model | 画像理解用モデル | 图像理解模型 |
| Role: embedding | Search model | 検索用モデル | 检索模型 |
| Role: organizer | Text organizing model | テキスト整形用モデル | 文本整理模型 |
| Vision unsupported | This model can't read images | このモデルは画像を読めません | 此模型无法读取图像 |
| Tokens used | Tokens used | 使用トークン数 | 使用的词元数 |

## 訳語の方針

- **日本語**：カタカナ語を無闇に増やさない。定着していない語（アーティファクト、チャンク、ロケータ）はUIに出さないか、平易な日本語に置き換える。
- **中文**：大陆简体の一般的なソフトウェア用語に合わせる（「资料」「工作区」「词元」など）。台湾繁体の語彙（「檔案」「詞元」）は使わない。
- **English**：内部の設計語（Chunk, Locator, Document）をUIに出さない。ユーザーが見るのは具体語（page, slide, segment）。
