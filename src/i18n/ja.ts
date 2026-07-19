import type { en } from "./en";

/** 日本語 — 標準的なデスクトップアプリ調。 */
export const ja: Record<keyof typeof en, string> = {
  // 共通
  cancel: "キャンセル",
  close: "閉じる",
  apply: "適用",

  // ホーム
  appTagline: "画面を録画し、フレームを編集し、美しいGIFに書き出します。",
  selectRegionRecord: "範囲を選択して録画",
  fps: "FPS",
  captureCursor: "カーソルをキャプチャ",
  continueEditing: "← 編集を続ける（{count}フレーム）",
  openProject: "プロジェクトを開く…",
  openFile: "開く（プロジェクト · GIF）",
  hotkeysHint: "ショートカット: F7 開始/一時停止 · F8 停止",
  browserPreviewNotice: "ブラウザプレビュー — 録画にはデスクトップアプリが必要です。",
  settingsTitle: "設定",

  // エディタ ツールバー
  home: "ホーム",
  play: "再生",
  pause: "一時停止",
  playPauseTitle: "再生/一時停止（Space）",
  delete: "削除",
  deleteSelectedTitle: "選択したフレームを削除（Del）",
  duplicate: "複製",
  duplicateTitle: "選択したフレームを複製",
  delay: "遅延",
  delayInputTitle: "1フレームあたりのミリ秒（10〜60000）",
  set: "適用",
  setDelayTitle: "選択したフレームに遅延を適用（最小10ms）",
  crop: "切り抜き",
  cropTitle: "切り抜き（Escで取り消し）",
  resize: "サイズ変更",
  resizeTitle: "サイズ変更",
  undoTitle: "元に戻す（Ctrl+Z）",
  redoTitle: "やり直し（Ctrl+Y）",
  dimensionsFrames: "{w}×{h} · {count}f",
  export: "書き出し",
  msPlaceholder: "ms",

  // エディタ ツールバー ツールチップ（アイコン専用ボタン）
  prevFrame: "前のフレーム",
  nextFrame: "次のフレーム",
  undo: "元に戻す",
  redo: "やり直し",
  tipHomeDesc: "スタート画面に戻ります",
  tipPrevDesc: "1フレーム戻ります",
  tipNextDesc: "1フレーム進みます",
  tipPlayDesc: "フレームを順番にプレビューします",
  tipDeleteDesc: "選択したフレームを削除します",
  tipDuplicateDesc: "選択したフレームの複製を挿入します",
  tipSetDelayDesc: "選択したフレームにこの遅延を適用します（10〜60000ms）",
  tipCropDesc: "プレビューをドラッグして切り抜き — Escで取り消し",
  tipResizeDesc: "全フレームを新しいサイズに変更します",
  tipUndoDesc: "直前の編集を取り消します",
  tipRedoDesc: "取り消した編集をやり直します",
  tipSaveDesc: ".voidgif プロジェクトとして保存します",
  tipExportDesc: "GIF・APNG・PNG・MP4に書き出します",

  // エディタ 録画を続ける
  continueRec: "録画を続ける",
  tipContinueRecDesc:
    "録画を続ける — 先頭・現在のフレームの後・末尾に新しい録画を挿入します",
  continueAtStart: "先頭に",
  continueAfterCurrent: "現在のフレームの後に",
  continueAtEnd: "末尾に",
  errRecordingTooLarge: "録画範囲が画面より大きいです。",

  // エディタ グループ
  group: "グループ化",
  ungroup: "グループ解除",
  tipGroupDesc: "選択した連続フレームをグループ化します",
  tipUngroupDesc: "選択したグループを解除します",

  // エディタ トリム / 統合 / 速度 / ループ (Wave 1)
  trimStatic: "静止トリム",
  tipTrimDesc: "先頭と末尾の動きのないフレームを自動で削除します",
  mergeDupes: "重複を統合",
  tipMergeDesc: "繰り返しのフレームを1つにまとめ、遅延を合算します",
  speed: "速度",
  tipSpeedDesc: "再生を速く・遅く — 選択フレーム、または全フレームに適用",
  loopTools: "ループ",
  tipLoopDesc: "ピンポン、末尾フレームの静止、つなぎ目プレビュー",
  trimmedFrames: "静止フレームを{count}個トリムしました",
  trimNothing: "トリムする静止区間はありません",
  mergedFrames: "重複フレームを{count}個統合しました",
  mergeNothing: "重複フレームは見つかりませんでした",
  speedTitle: "再生速度",
  speedApplySelection: "選択中の{count}フレームに適用",
  speedApplyAll: "すべてのフレームに適用",
  speedClampNote: "20ms未満の遅延は一部のブラウザ・ビューアで補正される場合があります。",
  loopPingpong: "ピンポン（往復再生）を作成",
  loopEndFreeze: "末尾フレームを1秒静止",
  loopSeamPreview: "つなぎ目をプレビュー",
  seamPreviewActive: "つなぎ目プレビュー",

  // 録画 続きモード
  sizeLocked: "サイズ固定",

  // エディタ 切り抜き / プレビュー
  applyCrop: "切り抜きを適用",
  dismissError: "閉じる",

  // エディタ 空状態 / エラー
  noRecordingLoaded: "録画が読み込まれていません。",
  backToHome: "ホームに戻る",
  errCropOutside: "切り抜き範囲が画像の外です。",
  errDelayRange: "遅延は10〜60000msの範囲で指定してください。",
  errSelectFramesFirst: "先にフレームを選択してから遅延を設定してください。",

  // エクスポート ダイアログ
  noteGifski: "gifski — 最高品質",
  noteApng: "可逆圧縮、24ビット",
  notePngSeq: "1フレームにつき1ファイル",
  noteMp4: "H.264 動画",
  quality: "品質",
  fastMode: "高速モード（品質低下、約3倍速）",
  widthLabel: "幅（px、空欄 = {size}）",
  widthPlaceholder: "例: 800",
  exportSourceSizeNote:
    "元のサイズ（{w}×{h}）で書き出します。サイズを変えるにはエディタで先にサイズ変更してください。",
  stageCollecting: "収集中",
  stageEstimating: "サイズ調整中",
  stageEncoding: "エンコード中",
  stageWriting: "書き込み中",
  stageDone: "完了",
  stageError: "エラー",
  exportFailed: "書き出しに失敗しました",
  savedMsg: "保存しました ✓ {message}",
  cancelExport: "書き出しを中止",
  exporting: "書き出し中…",

  // 書き出し: サイズ推定 + プラットフォームプリセット
  estSize: "推定サイズ",
  estimatingSize: "計算中…",
  fitExport: "目標サイズで書き出し",
  fitChosen: "品質 {quality} · {width}px · {size}",
  fitOverTarget: "{target}に収まりませんでした — 最小で {size}",

  // 書き出し: 結果を共有
  copyToClipboard: "クリップボードにコピー",
  copiedToClipboard: "コピーしました ✓",
  revealInFolder: "フォルダーで表示",

  // クラッシュ復元
  recoverTitle: "保存されていない作業があります",
  recoverBody: "{count}フレーム · {time} に保存",
  recoverAction: "復元",
  recoverDiscard: "破棄",

  // 未保存の作業ガード（新規録画 / プロジェクトを開く）
  unsavedTitle: "未保存の作業があります",
  unsavedBodyRecord:
    "新しく録画すると、編集中の {count} フレームは失われます。先に保存しますか？",
  unsavedBodyOpen:
    "プロジェクトを開くと、編集中の {count} フレームは失われます。先に保存しますか？",
  unsavedSaveContinue: "保存して続行",
  unsavedContinue: "保存せず続行",

  // サイズ変更 ダイアログ
  width: "幅",
  height: "高さ",
  lockAspect: "縦横比を固定",
  errInvalidSize: "正しいサイズを入力してください。",

  // プロジェクト保存ボタン
  save: "保存",
  saving: "保存中…",
  saved: "保存しました ✓",
  saveFailed: "失敗 ✕",
  saveProjectTitle: "プロジェクトを保存（.voidgif）",

  // 録画パネル
  startRecordingTitle: "録画開始（F7）",
  cursor: "カーソル",
  fullScreen: "全画面",
  recorderHint: "F7 開始 · F8 閉じる",
  closeTitle: "閉じる（F8）",
  resumeTitle: "再開（F7）",
  pauseTitle: "一時停止（F7）",
  stop: "停止",
  stopEditTitle: "停止して編集（F8）",
  discardTitle: "録画を破棄",
  discardConfirm: "破棄しますか？",

  // オンボーディング
  onboardingSubtitle: "かんたんに設定しましょう — いつでも変更できます。",
  theme: "テーマ",
  themeDark: "ダーク",
  themeLight: "ライト",
  language: "言語",
  languageSystem: "システムの既定",
  continueButton: "続ける",

  // 設定ダイアログ
  defaultFps: "デフォルトFPS",
  captureCursorDefault: "デフォルトでカーソルをキャプチャ",
};
