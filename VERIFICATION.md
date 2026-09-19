# 動作確認

## v0.2.0（2026-09-19）

Linux ARM64 / Docker / Rust 1.98.1 / Topcoat 0.8.1 / Playwright 1.63.0 / Chromiumで確認しました。

| チェック | 結果 |
| --- | --- |
| Rust単体・DBテスト | 15 / 15 成功 |
| Playwright E2E | 28 / 28 成功（14シナリオ × デスクトップ・スマートフォン） |
| Playwright画像回帰 | 18 / 18 成功（9状態 × 2サイズ） |
| TypeScript型チェック・Viteビルド | 成功。Dockerビルド時にも実行 |
| `cargo fmt --check` | 成功 |
| `cargo clippy --locked --all-targets -- -D warnings` | 成功 |
| HTTPスモークテスト | 独立した一時DBで10項目群成功 |

`./scripts/test.sh` の一括実行で全61件が成功。失敗・スキップ・再試行は0です。

E2Eには追加・完了・復元・削除、フィルター、再読み込みとサーバー再起動後の保持、入力制限、HTML文字列の安全な表示、不正なリクエストとクロスオリジンPOSTの拒否、同時書き込みを含みます。v0.2.0では以下を追加しました。

- 削除確認、キャンセル、Escape、フォーカスの復帰とダイアログ内への維持
- スキップリンク、`/` キーによる入力欄への移動、入力中の `/` とEnter送信
- 空・一部完了・全完了・未完了への復帰に応じた完了率の読み上げ値
- 直接URLとブラウザーの戻る・進むによるフィルターの保持
- axeによるWCAG 2 A/AA・2.1 AAの自動検査。空・一覧・削除ダイアログを2サイズで検査し、違反0

画像基準はモノクロのデザインに更新しました。9状態は空、混在一覧、未完了、完了、全完了、入力エラー、長いタイトル、削除ダイアログ、入力フォーカスです。1280×900と390×844で撮影し、横にはみ出さないことも検証します。目視確認後、基準を更新しない通常実行でも18件すべて合格しました。

意図的に見出しの色を変更した検出確認では **2,115ピクセル**の差分で失敗しました。18枚の基準PNGのSHA-256は前後で一致。一時的な検証コードを削除してから、全61件の通常テストを再実行しています。

ローカルのDockerアプリもv0.2.0に再作成し、既存DBのID・タイトル・完了状態のハッシュが更新前後で一致することを確認しました。ブラウザーで新デザインと既存Todoを確認し、`/health` とレポートもHTTP 200でした。README用画像は個人データを含まないテスト用一覧から取得しています。

## 証跡と再現方法

- [verification/test-summary.json](verification/test-summary.json): 最終結果の要約
- [画像差分の検出証跡](verification/regression-proof/README.md): 期待・実際・差分画像
- [基準PNG 18枚](tests/playwright/__screenshots__/linux-arm64/)
- `playwright-report/index.html`: 実行時に生成するHTMLレポート（Git管理対象外。`./scripts/test.sh report` で閲覧）
- [README](README.md): 起動・テスト・画像基準更新の手順

各Playwrightテストは専用のRustプロセスと一時SQLite DBを使用します。外部ネットワークを無効にしたテストコンテナで実行し、起動中のアプリのデータを利用しません。

対象はChromiumとLinux ARM64です。WebKit・Firefox・実機端末は未検証です。画像比較には同じOS・CPU・ブラウザー・フォント環境が必要です。アクセシビリティの検証範囲は自動検査と記載したキーボード操作です。

## v0.1.0

初回リリースではRust15件、E2E18件、画像回帰14件が成功。サーバー描画UIの検証記録は [v0.1.0のVERIFICATION.md](https://github.com/Sparklingstadt/topcoat-todo/blob/v0.1.0/VERIFICATION.md) に保存しています。
