# 動作確認

## 自動テストの追加（2026-09-19）

- Rust単体・DBテスト: **15 / 15 成功**（7件の入力・ルート処理、8件のSQLite処理）
- Playwright E2E: **18 / 18 成功**（デスクトップ9件、スマートフォン9件）
- Playwright画像回帰: **14 / 14 成功**（7画面 × 2サイズ）
- 通常実行の失敗・スキップ・再試行: **0**
- `cargo fmt --check` と `cargo clippy --locked --all-targets -- -D warnings`: 成功
- `./scripts/test.sh` による一括実行も成功

画像比較の基準はLinux ARM64 / Playwright 1.63.0 / Chromiumで生成。初回作成後、基準を更新しない通常実行でもすべて合格しました。見出しの色だけを一時変更した検出確認では587ピクセルの差分で失敗し、復元後のE2E・画像比較32件はすべて再合格しました。14枚の基準画像のハッシュは検出確認前後で同一です。

テストはケースごとに新しいRustプロセスと一時SQLite DBを作り、既存アプリのDBを利用しません。サーバーを実際に再起動するE2Eも含まれます。

- `verification/test-summary.json`: 最終結果の要約
- `playwright-report/index.html`: 実行時に生成するHTMLレポート（Git管理対象外。`./scripts/test.sh report` で閲覧）
- `tests/playwright/__screenshots__/linux-arm64/`: 基準PNG 14枚
- `verification/regression-proof/`: 意図的な画像差分の検出証跡

今回のブラウザーテストはChromiumのみ。WebKit・Firefoxおよび実機端末での確認は含みません。画像基準は同じOS・CPU・ブラウザー・フォント環境で比較します。

## 初回実装時の動作確認

2026-09-19、macOS上のDocker Desktop（Linux ARM64）、Rust 1.98.1、Topcoat 0.8.1で確認しました。

- `cargo fmt --check`、`cargo clippy --locked -- -D warnings`：成功
- Dockerのリリースビルドとlocalhost:3000での起動：成功
- HTTPテスト：10項目群すべて成功。日本語Todoの追加、完了と未完了への変更、絞り込み、削除、空白・文字数制限、HTMLエスケープ、クロスオリジン更新の拒否、8件の同時追加、不正なIDを確認
- ChromeでEnterキー・追加ボタン・完了ボタン・絞り込みリンク・削除ボタンを実操作：成功
- Docker再起動とイメージ再ビルドによるコンテナ再作成後も、2件のTodoと完了状態が維持されることを確認
- 1280×900と390×844で表示を確認。390px幅で文書幅も390px、横方向のはみ出しなし
- axe-core 4.12.1による最終画面の自動検査：42項目合格、違反0、判定保留0（自動検査の範囲）
- 検証用ブラウザーのページエラー：なし

`preview-desktop.png` と `preview-mobile.png` は2件の検証用Todoを表示した画面例です。検証後、それらのTodoは削除しました。

検証の再現方法、起動・停止・開発手順は [README.md](README.md) を参照してください。
