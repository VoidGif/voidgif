# VoidGif — Microsoft Store 제출 가이드 (오너용 체크리스트)

이 문서 하나만 따라가면 VoidGif를 Microsoft Store에 제출할 수 있습니다.
**오너가 직접 해야 하는 일은 딱 두 가지** — ① Partner Center 계정 만들고
② 마지막에 "제출" 클릭 — 이고, 나머지 준비물(패키지 빌드 스크립트, 등록 문구,
개인정보처리방침, 스크린샷)은 이 `store/` 폴더에 모두 준비되어 있습니다.

---

## 왜 MSIX 방식인가 (3줄 요약)

1. **MSIX를 스토어에 올리면 Microsoft가 무료로 서명해 줍니다** — 유료 코드 서명
   인증서(연 15~50만 원)가 전혀 필요 없습니다.
2. 다른 방식인 **EXE/MSI 제출은 반드시 CA 발급 유료 인증서로 직접 서명**해야
   하고(자가 서명 불가), 설치 파일을 직접 호스팅하고 업데이트도 직접 관리해야
   합니다.
3. 따라서 유료 인증서가 없는 1인 개발자에게는 **MSIX가 유일하게 합리적인 경로**
   입니다. (근거: Microsoft Learn "Code signing options", "App package
   requirements for MSIX/MSI-EXE" — 이 문서 맨 아래 링크 참고)

---

## 준비 상태 요약 (이미 만들어 둔 것)

| 파일 / 폴더 | 용도 | 어디에 쓰나 |
| --- | --- | --- |
| `store/msix/AppxManifest.xml` | MSIX 패키지 매니페스트 (플레이스홀더 3개) | Step 4에서 값 채워 빌드 |
| `store/msix/build-msix.ps1` | 릴리스 exe → `.msix` 자동 빌드 스크립트 | Step 4에서 실행 |
| `store/listing/en.md`, `ko.md`, `ja.md` | 영어/한국어/일본어 등록 문구 | Step 8에서 붙여넣기 |
| `store/privacy-policy.md` | 개인정보처리방침 (영/한) | Step 5에서 공개 URL로 게시 |
| `store/screenshots/` | 스토어 스크린샷 (1366×768 PNG) | Step 8에서 업로드 |

> **소스 코드는 수정하지 않았습니다.** MSIX의 패키지 정체성(Identity)은
> Partner Center 값에서 오므로 `tauri.conf.json`의 `identifier`
> (`com.voidgif.desktop`)를 바꿀 필요가 없습니다. 그대로 두세요.

---

## Step 0 — 릴리스 빌드 준비 (제출 직전에 새로 빌드)

제출용 패키지는 **최신 소스로 새로 빌드한 exe**여야 합니다.

1. 실행 중인 VoidGif가 있으면 **모두 종료**하세요. (exe 파일이 잠겨 있으면 빌드가
   `os error 5 (액세스 거부)`로 실패합니다. 실제로 준비 과정에서 이 잠금 때문에
   기존 빌드로 패키징을 검증했습니다.)
2. 프런트엔드 + 릴리스 빌드:
   ```powershell
   npm install            # 최초 1회
   npm run build          # dist/ 생성
   cargo build --release --features custom-protocol --manifest-path src-tauri\Cargo.toml
   ```
   > ⚠️ `--features custom-protocol` 필수. 빼고 빌드하면 exe가 번들 대신
   > 개발 서버 URL(localhost:1420)을 로드해서 사용자 PC에서 흰 화면만 나옵니다.
   > (`npx tauri build`로 빌드하면 자동으로 켜지므로 이 걱정이 없습니다.)
3. 결과물: `src-tauri\target\release\voidgif.exe` (약 15 MB, 단일 실행 파일).
   > WebView2 런타임에만 의존합니다. Windows 11에는 기본 내장, Windows 10에도
   > 대부분 설치되어 있어 별도 동봉이 필요 없습니다.

---

## Step 1 — Partner Center 개인 개발자 계정 등록 ($19)

1. https://partner.microsoft.com/dashboard 접속 → **Windows & Xbox** 프로그램 등록.
2. 계정 유형: **개인(Individual)** 선택. (1회성 등록비 약 **$19**, 이후 연회비
   없음.)
3. 이름·주소·결제 정보 입력 후 결제. 개인 계정은 보통 즉시~며칠 내 승인됩니다.
   > **오너가 직접 해야 하는 단계입니다** (결제·본인 정보 입력이라 대행 불가).

---

## Step 2 — 앱 이름 "VoidGif" 예약

1. 대시보드 → **Apps and games** → **New product** → **MSIX or PWA app** 선택.
   (⚠️ EXE/MSI가 아니라 **MSIX** 계열을 고르세요.)
2. 이름에 **`VoidGif`** 입력 → **Check availability** → 사용 가능하면
   **Reserve product name**.
   > 이름 예약은 개발 전이라도 가능하며, 예약하면 그 앱의 관리 화면이 열립니다.

---

## Step 3 — Identity 값 3개 복사 (매니페스트에 넣을 값)

예약이 끝나면 그 앱의 관리 화면에서 정체성 값을 확인합니다.

1. 앱 관리 화면 → **Product management** → **Product identity**
   (또는 "View app identity details").
2. 아래 **세 값**을 복사해 두세요 (대소문자·공백까지 정확히):

   | Partner Center 항목 | 매니페스트 플레이스홀더 | 예시 형식 |
   | --- | --- | --- |
   | Package/Identity/**Name** | `{{IDENTITY_NAME}}` | `1234ABCD.VoidGif` |
   | Package/Identity/**Publisher** | `{{PUBLISHER}}` | `CN=XXXXXXXX-XXXX-...` |
   | Package/Properties/**PublisherDisplayName** | `{{PUBLISHER_DISPLAY_NAME}}` | 판매자 표시 이름 |

---

## Step 4 — 값 넣고 `.msix` 빌드

Step 3의 세 값을 스크립트에 넘겨 실행합니다 (매니페스트 원본은 그대로 두고
스크립트가 복사본에 값을 채워 넣습니다).

```powershell
cd D:\git\VoidGif\store\msix
.\build-msix.ps1 `
    -IdentityName        "여기에_Name값" `
    -Publisher           "여기에_Publisher값(CN=...)" `
    -PublisherDisplayName "여기에_PublisherDisplayName값"
```

- 스크립트가 하는 일: 릴리스 exe를 `VoidGif.exe`로 스테이징 → `icon.png`에서
  타일/로고 PNG 자동 생성 → `resources.pri` 생성 → **makeappx로 `.msix` 패킹**.
- 결과물: `store\msix\VoidGif_0.1.0.0_x64.msix` ← **이 파일을 업로드**합니다.
- **서명하지 마세요.** 이 `.msix`는 일부러 **서명하지 않은** 상태입니다. 스토어가
  인증 과정에서 자동으로 Microsoft 인증서로 서명합니다.
  (로컬에서 설치해 눈으로 확인만 하고 싶다면 `-SelfSignForLocalTest` 옵션으로
  임시 자가 서명본을 만들 수 있지만, **그 서명본은 업로드 금지**입니다. 서명되지
  않은 MSIX는 개발자 모드/자가 서명 없이는 설치되지 않는다는 점만 알아두세요.)

> **검증 완료:** 이 스크립트는 준비 과정에서 실제로 실행해 유효한
> `VoidGif_0.1.0.0_x64.msix`(약 5 MB, 13개 파일 + `resources.pri`)를
> 만들어 냈고, `makeappx unpack`으로 매니페스트·자산까지 확인했습니다.

---

## Step 5 — 개인정보처리방침을 공개 URL로 게시 (필수) ✅ 완료

Microsoft Store 정책 10.5.1에 따라 **Win32/Desktop Bridge 앱은 개인정보처리방침
URL이 반드시 필요**합니다(데이터를 수집하지 않아도 요구됨).

**✅ 이미 게시되어 있습니다 (2026-07-18):**
- 방침 URL: **https://voidgif.github.io/privacy/** ← Step 7의 Properties 화면에
  이 URL을 그대로 입력하면 됩니다.
- 호스팅: GitHub 조직 `VoidGif`의 `voidgif.github.io` 리포 (GitHub Pages,
  커밋 author는 브랜드 명의 — 개인 계정 노출 없음).
- 문의처는 이메일 대신 GitHub Issues 링크 사용 (개인 이메일 비노출).
- 내용 수정이 필요하면 그 리포의 `privacy/index.html`을 고쳐 푸시하면 즉시 반영.

---

## Step 6 — 패키지 업로드 (Packages 화면)

1. 앱 관리 화면 → 새 **Submission** 시작 → **Packages** 탭.
2. `store\msix\VoidGif_0.1.0.0_x64.msix`를 드래그해 업로드.
3. 업로드 후 매니페스트의 Identity가 Partner Center 값과 일치하면 초록색으로
   검증됩니다. (불일치 시 Step 3 값과 대소문자·공백을 다시 대조하세요.)
   - 지원 언어(en/ko/ja)와 최소 OS 버전(Windows 10 1809)이 자동 인식됩니다.

---

## Step 7 — Availability / Properties / Age ratings

각 탭을 채웁니다.

- **Availability(Pricing and availability)**: 가격 **Free** 권장, 배포 시장은
  **All markets**(또는 원하는 국가), 공개 여부 설정.
- **Properties**:
  - **Category**: `Multimedia design`(또는 `Developer tools`) 등 적절히 선택.
  - **Privacy policy URL**: Step 5의 URL 입력. ← **필수**
  - 시스템 요구사항은 비워도 됩니다.
- **Age ratings**: IARC 설문에 답하면 등급이 자동 산정됩니다. VoidGif는 성인
  콘텐츠·데이터 수집·사용자 상호작용이 없으므로 대부분 "아니오" → 전연령(3+)로
  나옵니다.

---

## Step 8 — 등록 정보(Store listing) 붙여넣기 + 스크린샷

언어마다 **Store listing** 페이지가 따로 있습니다. 최소 한 개(영어 권장) 필수이며,
한국어·일본어도 채우면 좋습니다.

1. **Add/Manage languages**에서 English(US), Korean, Japanese 추가.
2. 각 언어 페이지에 대응하는 파일 내용을 붙여넣습니다:
   - English → `store/listing/en.md`
   - 한국어 → `store/listing/ko.md`
   - 日本語 → `store/listing/ja.md`
   각 파일 안의 항목(제품 이름·짧은 설명·설명·제품 기능·검색어·새로운 기능)을
   해당 필드에 그대로 옮기세요.
3. **Screenshots**: `store/screenshots/`의 PNG를 업로드합니다.
   - 최소 1장 필수, 4장 이상 권장, 데스크톱 규격 **1366×768 이상** PNG.
   - 스크린샷은 언어별로 각각 업로드해야 합니다(같은 이미지를 재사용해도 됨).
4. (선택) **Store logos**: 1:1 300×300 아이콘을 올리면 더 깔끔하게 표시됩니다.
   `src-tauri/icons/Square310x310Logo.png` 등을 활용할 수 있습니다.

---

## Step 9 — 심사 제출

1. 모든 탭에 초록 체크가 뜨면 **Submit to the Store** (또는 Store listing 페이지의
   **Publish**) 클릭.
   > **여기서 오너가 "제출"을 누르면 끝입니다.**
2. 인증(certification)은 보통 몇 시간~며칠 소요됩니다. 통과하면 스토어에 게시되고,
   반려되면 사유가 메일/대시보드로 옵니다.

---

## Step 10 — 이후(업데이트 방법)

- 새 버전을 낼 때: 소스 수정 → Step 0 재빌드 → `build-msix.ps1 -Version 0.1.1.0`
  처럼 **버전을 올려** 빌드(4번째 자리는 항상 `0` 유지) → 새 Submission에 업로드.
- MSIX는 스토어가 자동 업데이트를 배포하므로, 기존 사용자는 별도 조치 없이
  새 버전을 받습니다.

---

## 알아두면 좋은 점 / 열린 이슈

- **GIF 인코더 gifski는 AGPL-3.0**입니다. Windows 빌드에서 gifski를 사용하는 것은
  문제 없으나(오픈소스 배포), 향후 **macOS(App Store) 빌드에는 AGPL 링크가
  불가**합니다. 소스에서 macOS 타깃은 MIT 계열 대체 인코더로 자동 분리되어 있으니
  Windows 제출에는 영향이 없습니다.
- **최소 OS 버전**은 매니페스트에서 Windows 10 1809(10.0.17763)로 설정했습니다.
  화면 캡처(Windows Graphics Capture) 안정성 기준입니다. 더 최신만 지원하려면
  `AppxManifest.xml`의 `MinVersion`을 올리면 됩니다.
- **파일 연결(.voidgif 더블클릭 열기)** 은 이번 제출에는 넣지 않았습니다(인증을
  단순하게 유지). 앱 내 CLI 열기는 그대로 동작하며, 원하면 다음 버전에서 매니페스트에
  파일 형식 연결을 추가할 수 있습니다.
- **연락처**: 개인정보처리방침의 문의처는 GitHub Issues 링크로 처리 완료
  (이메일 불필요). 스토어의 Support contact info 필드는 데스크톱 앱은 선택
  사항이므로 비워도 되고, 원하면 같은 Issues URL을 넣으면 됩니다.

---

## 근거가 된 Microsoft 공식 문서

- Code signing options for Windows app developers —
  https://learn.microsoft.com/windows/apps/package-and-deploy/code-signing-options
- App package requirements for MSIX app (Store가 자동 서명) —
  https://learn.microsoft.com/windows/apps/publish/publish-your-app/msix/app-package-requirements
- App package requirements for MSI/EXE app (유료 인증서·무인 설치 요구) —
  https://learn.microsoft.com/windows/apps/publish/publish-your-app/msi/app-package-requirements
- Create an MSIX package with MakeAppx.exe —
  https://learn.microsoft.com/windows/msix/package/create-app-package-with-makeappx-tool
- Generating MSIX package components (runFullTrust / FullTrustApplication) —
  https://learn.microsoft.com/windows/msix/desktop/desktop-to-uwp-manual-conversion
- Microsoft Store Policies 10.5.1 (개인정보처리방침 필수) —
  https://learn.microsoft.com/windows/apps/publish/store-policies
- Screenshots/이미지 규격 (데스크톱 1366×768 이상) —
  https://learn.microsoft.com/windows/apps/publish/publish-your-app/msix/screenshots-and-images
