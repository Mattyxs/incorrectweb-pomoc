#!/usr/bin/env bash
# IW: builds the macOS app (arm64) and its disk image on a Mac WITHOUT Xcode (Command Line Tools only), for when the
# workflow (.github/workflows/incorrectweb-build.yml, job build-macos) cannot run.
#
# What is built here: the Rust library and the service (cargo, as build.py), the Dart code (flutter assemble ->
# App.framework). What is taken from the disk image of an earlier release made by the workflow (BASE, from GitHub):
# the native runner (Swift of flutter/macos/Runner, needs xcodebuild), the plugin frameworks (CocoaPods), FlutterMacOS and
# the compiled MainMenu.nib (needs ibtool). That is only right while those sources are the same as in BASE, so the script
# stops when flutter/macos (other than Info.plist and MainMenu.xib), flutter/pubspec.yaml or flutter/pubspec.lock changed
# since BASE. Menu titles of MainMenu.xib are written into the nib (res/macos/nib_titles.py), Info.plist is the one of the
# repository. Signed ad hoc, disk image as in the workflow.
#
#   res/macos/build-local.sh <BASE release, e.g. 1.4.9-iw.10>     -> out/IncorrectWebPomoc-<IW_VERSION>-aarch64.dmg
# Needs: Flutter 3.24.5 (FLUTTER_ROOT, default ~/dev/flutter, patched as in the workflow), vcpkg (VCPKG_ROOT, default
# ~/dev/vcpkg, `vcpkg install --triplet arm64-osx` done), nasm 2.16 on PATH, rustup with 1.81, create-dmg, gh.
# Without Rosetta: gen_snapshot_arm64 of Flutter 3.24.5 is an x86_64 program; the arm64 gen_snapshot of the Dart SDK of the
# same Flutter makes the same snapshot (same hash and features) and may take its place:
#   cp $FLUTTER_ROOT/bin/cache/dart-sdk/bin/utils/gen_snapshot $FLUTTER_ROOT/bin/cache/artifacts/engine/darwin-x64-release/gen_snapshot_arm64
# SKIP_CARGO=1 uses target/release as it is.
set -euo pipefail

BASE=${1:?Podaj wydanie bazowe, np. 1.4.9-iw.10}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT"
FLUTTER_ROOT=${FLUTTER_ROOT:-$HOME/dev/flutter}
export VCPKG_ROOT=${VCPKG_ROOT:-$HOME/dev/vcpkg}
export PATH="$FLUTTER_ROOT/bin:$PATH" MACOSX_DEPLOYMENT_TARGET=12.3
VER=$(sed -n 's/^pub const IW_VERSION: &str = "\(.*\)";/\1/p' src/iw/mod.rs)
[ -n "$VER" ] || { echo "Brak IW_VERSION w src/iw/mod.rs" >&2; exit 1; }
grep -q "<string>$VER</string>" flutter/macos/Runner/Info.plist || { echo "Info.plist: IWVersion inna niż $VER" >&2; exit 1; }

echo "→ natywna część bez zmian od $BASE?"
changed=$(git diff --name-only "$BASE" HEAD -- flutter/macos flutter/pubspec.yaml flutter/pubspec.lock \
  | grep -v -e '^flutter/macos/Runner/Info.plist$' -e '^flutter/macos/Runner/Base.lproj/MainMenu.xib$' \
            -e '^flutter/macos/Runner/bridge_generated.h$' || true)
dirty=$(git status --porcelain -- flutter/macos flutter/pubspec.yaml | grep -v -e 'Runner/Info.plist$' -e 'MainMenu.xib$' -e 'bridge_generated.h$' || true)
if [ -n "$changed$dirty" ]; then
  echo "Zmienione od $BASE (potrzebny build z Xcode, workflow):" >&2; echo "$changed$dirty" >&2; exit 1
fi

if [ -z "${SKIP_CARGO:-}" ]; then
  echo "→ cargo (Rust 1.81, macOS 12.3)"
  # cidre (feature screencapturekit) runs xcodebuild in its build script: a stand-in builds its pomace libraries
  PATH="$ROOT/res/macos/xcodebuild-shim:$PATH" cargo +1.81 build --locked --features flutter,hwcodec,unix-file-copy-paste,screencapturekit --release
fi
for f in target/release/liblibrustdesk.dylib target/release/service; do
  [ "$(lipo -archs "$f")" = arm64 ] || { echo "$f nie jest arm64" >&2; exit 1; }
done

echo "→ flutter assemble (App.framework)"
FOUT="$ROOT/flutter/build/iw-macos-arm64"
rm -rf "$FOUT" && mkdir -p "$FOUT"
(cd flutter && flutter pub get >/dev/null && flutter assemble --no-version-check -dTargetPlatform=darwin -dDarwinArchs=arm64 \
  -dTargetFile=lib/main.dart -dBuildMode=release -dTreeShakeIcons=true -dDartObfuscation=false -dSplitDebugInfo= \
  -dTrackWidgetCreation=false -dAction=build --DartDefines= --ExtraGenSnapshotOptions= --ExtraFrontEndOptions= \
  --output="$FOUT" release_macos_bundle_flutter_assets)
git checkout -- flutter/pubspec.lock 2>/dev/null || true

WORK=$(mktemp -d)
trap 'hdiutil detach "$WORK/mnt" >/dev/null 2>&1 || true; rm -rf "$WORK"' EXIT
echo "→ wydanie bazowe $BASE (GitHub)"
gh release download "$BASE" --repo Mattyxs/incorrectweb-pomoc --dir "$WORK" --pattern "IncorrectWebPomoc-$BASE-aarch64.dmg" --pattern SHA256SUMS
(cd "$WORK" && shasum -a 256 -c SHA256SUMS --ignore-missing)
hdiutil attach -nobrowse -readonly -mountpoint "$WORK/mnt" "$WORK/IncorrectWebPomoc-$BASE-aarch64.dmg" >/dev/null
mkdir -p "$WORK/app"
ditto "$WORK/mnt/IncorrectWebPomoc.app" "$WORK/app/IncorrectWebPomoc.app"
hdiutil detach "$WORK/mnt" >/dev/null
APP="$WORK/app/IncorrectWebPomoc.app"
C="$APP/Contents"

echo "→ składanie $VER"
rm -rf "$C/_CodeSignature" "$C/Frameworks/App.framework"
ditto "$FOUT/App.framework" "$C/Frameworks/App.framework"
cp target/release/liblibrustdesk.dylib "$C/Frameworks/liblibrustdesk.dylib"
cp target/release/service "$C/MacOS/service"
cp flutter/macos/Runner/AppIcon.icns "$C/Resources/AppIcon.icns"
cp -R res/macos/en.lproj res/macos/pl.lproj "$C/Resources/"
git show "$BASE:flutter/macos/Runner/Base.lproj/MainMenu.xib" >"$WORK/base.xib"
python3 res/macos/nib_titles.py "$C/Resources/Base.lproj/MainMenu.nib" "$WORK/base.xib" flutter/macos/Runner/Base.lproj/MainMenu.xib
python3 - "$C/Info.plist" flutter/macos/Runner/Info.plist flutter/macos/Runner/Configs/AppInfo.xcconfig flutter/pubspec.yaml <<'PY'
import plistlib, re, sys
built, src, xcconfig, pubspec = sys.argv[1:5]
base = plistlib.load(open(built, 'rb'))
conf = dict(re.findall(r'^(\w+) = (.*)$', open(xcconfig, encoding='utf-8').read(), re.M))
name, number = re.search(r'^version: ([\d.]+)\+(\d+)', open(pubspec, encoding='utf-8').read(), re.M).groups()
subst = {'DEVELOPMENT_LANGUAGE': 'en', 'EXECUTABLE_NAME': conf['PRODUCT_NAME'], 'PRODUCT_NAME': conf['PRODUCT_NAME'],
         'PRODUCT_BUNDLE_IDENTIFIER': conf['PRODUCT_BUNDLE_IDENTIFIER'], 'PRODUCT_COPYRIGHT': conf['PRODUCT_COPYRIGHT'],
         'FLUTTER_BUILD_NAME': name, 'FLUTTER_BUILD_NUMBER': number, 'MACOSX_DEPLOYMENT_TARGET': '12.3'}
text = re.sub(r'\$\((\w+)\)', lambda m: subst[m.group(1)], open(src, encoding='utf-8').read())
new = plistlib.loads(text.encode('utf-8'))
for k in ('BuildMachineOSBuild', 'CFBundleSupportedPlatforms', 'DTCompiler', 'DTPlatformBuild', 'DTPlatformName',
          'DTPlatformVersion', 'DTSDKBuild', 'DTSDKName', 'DTXcode', 'DTXcodeBuild'):
    if k in base:
        new[k] = base[k]
assert new['CFBundleExecutable'] == base['CFBundleExecutable'] and new['CFBundleIdentifier'] == base['CFBundleIdentifier']
plistlib.dump(new, open(built, 'wb'))
PY

echo "→ podpis ad hoc"
codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict "$APP"

echo "→ obraz dysku"
mkdir -p out
DMG="out/IncorrectWebPomoc-$VER-aarch64.dmg"
rm -f "$DMG"
create-dmg \
  --volname "GoConnect Pomoc" \
  --volicon "$C/Resources/AppIcon.icns" \
  --background res/installer/dmg-background.tiff \
  --window-size 660 428 \
  --icon-size 128 \
  --icon "IncorrectWebPomoc.app" 180 180 \
  --hide-extension "IncorrectWebPomoc.app" \
  --app-drop-link 480 180 \
  "$DMG" "$APP"
shasum -a 256 "$DMG"
