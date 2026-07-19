import type { en } from "./en";

/** 한국어 — 해요체 간결형. */
export const ko: Record<keyof typeof en, string> = {
  // 공통
  cancel: "취소",
  close: "닫기",
  apply: "적용",

  // 홈
  appTagline: "화면을 녹화하고, 프레임을 편집하고, 멋진 GIF로 내보내요.",
  selectRegionRecord: "영역 선택 후 녹화",
  fps: "FPS",
  captureCursor: "커서 캡처",
  continueEditing: "← 이어서 편집 ({count}프레임)",
  openProject: "프로젝트 열기…",
  openFile: "열기 (프로젝트 · GIF)",
  hotkeysHint: "단축키: F7 시작/일시정지 · F8 정지",
  browserPreviewNotice: "브라우저 미리보기 — 녹화는 데스크톱 앱에서만 돼요.",
  settingsTitle: "설정",

  // 편집기 툴바
  home: "홈",
  play: "재생",
  pause: "일시정지",
  playPauseTitle: "재생/일시정지 (Space)",
  delete: "삭제",
  deleteSelectedTitle: "선택 프레임 삭제 (Del)",
  duplicate: "복제",
  duplicateTitle: "선택 프레임 복제",
  delay: "지연",
  delayInputTitle: "프레임당 밀리초 (10–60000)",
  set: "적용",
  setDelayTitle: "선택 프레임에 지연 적용 (최소 10ms)",
  crop: "자르기",
  cropTitle: "자르기 (Esc로 취소)",
  resize: "크기 조절",
  resizeTitle: "크기 조절",
  undoTitle: "실행 취소 (Ctrl+Z)",
  redoTitle: "다시 실행 (Ctrl+Y)",
  dimensionsFrames: "{w}×{h} · {count}f",
  export: "내보내기",
  msPlaceholder: "ms",

  // 편집기 툴바 툴팁 (아이콘 전용 버튼)
  prevFrame: "이전 프레임",
  nextFrame: "다음 프레임",
  undo: "실행 취소",
  redo: "다시 실행",
  tipHomeDesc: "시작 화면으로 돌아가요",
  tipPrevDesc: "한 프레임 뒤로 이동해요",
  tipNextDesc: "한 프레임 앞으로 이동해요",
  tipPlayDesc: "프레임을 순서대로 미리봐요",
  tipDeleteDesc: "선택한 프레임을 지워요",
  tipDuplicateDesc: "선택한 프레임을 복사해 넣어요",
  tipSetDelayDesc: "선택한 프레임에 이 지연을 적용해요 (10–60000ms)",
  tipCropDesc: "미리보기에서 드래그해 잘라요 — Esc로 취소",
  tipResizeDesc: "모든 프레임 크기를 바꿔요",
  tipUndoDesc: "마지막 편집을 되돌려요",
  tipRedoDesc: "되돌린 편집을 다시 적용해요",
  tipSaveDesc: ".voidgif 프로젝트로 저장해요",
  tipExportDesc: "GIF·APNG·PNG·MP4로 내보내요",

  // 편집기 이어서 녹화
  continueRec: "이어서 녹화",
  tipContinueRecDesc:
    "이어서 녹화 — 맨 앞·현재 프레임 뒤·맨 뒤에 새 녹화를 끼워 넣어요",
  continueAtStart: "맨 앞에",
  continueAfterCurrent: "현재 프레임 뒤에",
  continueAtEnd: "맨 뒤에",
  errRecordingTooLarge: "녹화 영역이 화면보다 커요.",

  // 편집기 그룹
  group: "그룹",
  ungroup: "그룹 해제",
  tipGroupDesc: "선택한 연속 프레임을 그룹으로 묶어요",
  tipUngroupDesc: "선택한 그룹을 해제해요",

  // 편집기 트림 / 병합 / 속도 / 루프 (Wave 1)
  trimStatic: "정지 트림",
  tipTrimDesc: "앞뒤의 멈춰 있는 프레임을 자동으로 없애요",
  mergeDupes: "중복 병합",
  tipMergeDesc: "똑같이 반복되는 프레임을 하나로 합치고 지연을 더해요",
  speed: "속도",
  tipSpeedDesc: "재생 속도를 빠르게·느리게 — 선택 프레임 또는 전체에 적용",
  loopTools: "루프",
  tipLoopDesc: "핑퐁, 끝 프레임 멈춤, 이음새 미리보기",
  trimmedFrames: "정지 프레임 {count}개 정리함",
  trimNothing: "정리할 정지 구간이 없어요",
  mergedFrames: "중복 프레임 {count}개 병합함",
  mergeNothing: "병합할 중복 프레임이 없어요",
  speedTitle: "재생 속도",
  speedApplySelection: "선택한 {count}개 프레임에 적용",
  speedApplyAll: "모든 프레임에 적용",
  speedClampNote: "20ms 미만 지연은 일부 브라우저·뷰어에서 보정될 수 있어요.",
  loopPingpong: "핑퐁 (왕복 재생) 만들기",
  loopEndFreeze: "끝 프레임 1초 멈춤",
  loopSeamPreview: "이음새 미리보기",
  seamPreviewActive: "이음새 미리보기",

  // 녹화 이어서 모드
  sizeLocked: "크기 고정",

  // 편집기 자르기 / 미리보기
  applyCrop: "자르기 적용",
  dismissError: "닫기",

  // 편집기 빈 상태 / 오류
  noRecordingLoaded: "불러온 녹화가 없어요.",
  backToHome: "홈으로",
  errCropOutside: "자르기 영역이 이미지 밖에 있어요.",
  errDelayRange: "지연은 10~60000ms 사이여야 해요.",
  errSelectFramesFirst: "먼저 프레임을 선택한 뒤 지연을 설정해요.",

  // 내보내기 대화상자
  noteGifski: "gifski — 최고 품질",
  noteApng: "무손실, 24비트",
  notePngSeq: "프레임당 파일 하나",
  noteMp4: "H.264 영상",
  quality: "품질",
  fastMode: "빠른 모드 (품질 낮음, 약 3배 빠름)",
  widthLabel: "너비 (px, 비우면 = {size})",
  widthPlaceholder: "예: 800",
  exportSourceSizeNote:
    "원본 크기({w}×{h})로 내보내요. 크기를 바꾸려면 편집기에서 먼저 크기 조절을 해요.",
  stageCollecting: "수집 중",
  stageEstimating: "크기 맞추는 중",
  stageEncoding: "인코딩 중",
  stageWriting: "저장 중",
  stageDone: "완료",
  stageError: "오류",
  exportFailed: "내보내기 실패",
  savedMsg: "저장 완료 ✓ {message}",
  cancelExport: "내보내기 취소",
  exporting: "내보내는 중…",

  // 내보내기: 크기 예측 + 플랫폼 프리셋
  estSize: "예상 크기",
  estimatingSize: "계산 중…",
  fitExport: "목표 크기로 내보내기",
  fitChosen: "품질 {quality} · {width}px · {size}",
  fitOverTarget: "{target}에 못 미쳤어요 — 가능한 최소는 {size}",

  // 내보내기: 결과 공유
  copyToClipboard: "클립보드에 복사",
  copiedToClipboard: "복사됨 ✓",
  revealInFolder: "폴더에서 보기",

  // 크래시 복구
  recoverTitle: "저장되지 않은 작업이 있어요",
  recoverBody: "{count}프레임 · {time}에 저장됨",
  recoverAction: "복구",
  recoverDiscard: "삭제",

  // 미저장 작업 확인 (새 녹화 / 프로젝트 열기)
  unsavedTitle: "저장하지 않은 작업이 있어요",
  unsavedBodyRecord:
    "새로 녹화하면 지금 편집 중인 프레임 {count}장이 사라져요. 먼저 저장할까요?",
  unsavedBodyOpen:
    "다른 프로젝트를 열면 지금 편집 중인 프레임 {count}장이 사라져요. 먼저 저장할까요?",
  unsavedSaveContinue: "저장 후 계속",
  unsavedContinue: "저장 안 하고 계속",

  // 크기 조절 대화상자
  width: "너비",
  height: "높이",
  lockAspect: "비율 고정",
  errInvalidSize: "올바른 크기를 입력해요.",

  // 프로젝트 저장 버튼
  save: "저장",
  saving: "저장 중…",
  saved: "저장 완료 ✓",
  saveFailed: "실패 ✕",
  saveProjectTitle: "프로젝트 저장 (.voidgif)",

  // 녹화 패널
  startRecordingTitle: "녹화 시작 (F7)",
  cursor: "커서",
  fullScreen: "전체 화면",
  recorderHint: "F7 시작 · F8 닫기",
  closeTitle: "닫기 (F8)",
  resumeTitle: "다시 시작 (F7)",
  pauseTitle: "일시정지 (F7)",
  stop: "정지",
  stopEditTitle: "정지 후 편집 (F8)",
  discardTitle: "녹화 버리기",
  discardConfirm: "버릴까요?",

  // 온보딩
  onboardingSubtitle: "간단히 설정해요 — 언제든 바꿀 수 있어요.",
  theme: "테마",
  themeDark: "다크",
  themeLight: "라이트",
  language: "언어",
  languageSystem: "시스템 기본값",
  continueButton: "계속",

  // 설정 대화상자
  defaultFps: "기본 FPS",
  captureCursorDefault: "기본으로 커서 캡처",
};
