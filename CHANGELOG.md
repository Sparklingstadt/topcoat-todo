# Changelog

## v0.2.0 — 2026-09-19

- shadcn/uiの実コンポーネントを導入。白・黒・グレー、Geist、細い罫線を使ったレスポンシブなダークデザインに刷新
- 完了率の表示、削除確認ダイアログ、キャンセルとフォーカス復帰、`/` 入力ショートカットを追加
- Topcoat・SQLite・既存フォームの303リダイレクトを維持。DBマイグレーションは不要
- React / Vite / Tailwind CSSのビルドをDockerに追加。ブラウザーのJavaScriptが必要になり、生成したUIとフォントはRustバイナリーに埋め込み
- Playwright E2Eを18件から28件に拡張。削除確認、キーボード、完了率、ブラウザー履歴、axeによるアクセシビリティ検査を追加
- 画像回帰を14件から18件に拡張し、新デザインの基準画像へ更新
- README、画面例、検証記録、第三者ライセンス表記を更新

## v0.1.0 — 2026-09-19

- Rust / Topcoat 0.8.1による日本語Todoアプリの初回リリース
- 追加・完了・未完了へ戻す・削除・絞り込み、SQLite永続化、Docker起動
- サーバー描画UI、レスポンシブ表示
- Rust単体・DBテスト15件、Playwright E2E18件、画像回帰14件
- MITライセンスで公開

[v0.1.0のリリース](https://github.com/Sparklingstadt/topcoat-todo/releases/tag/v0.1.0)
