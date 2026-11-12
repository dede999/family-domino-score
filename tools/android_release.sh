#!/bin/sh
# Gera o app Android assinado para a Play Store (.aab) e para instalar direto
# (.apk), em dist/. Rodar no Mac, com a chave de tools/android_key.sh.
#
#   sh tools/android_release.sh
#
# Antes de cada envio à loja, subir a versão no Cargo.toml: o número da
# versão do Android (versionCode) sai dela, e a loja recusa um número repetido.
set -eu
cd "$(dirname "$0")/.."

A=/Volumes/KINGSTON/Android
export JAVA_HOME="$A/jdk/Contents/Home"
export ANDROID_HOME="$A/sdk"
export ANDROID_NDK_HOME="$A/sdk/ndk/28.2.13676358"
export GRADLE_USER_HOME="$A/gradle"
export ANDROID_USER_HOME="$A/user"
export TMPDIR=/Volumes/KINGSTON/.cargo-build/tmp

sh tools/android_key.sh conferir >/dev/null

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
CODE=$(echo "$VERSION" | awk -F. '{ print $1 * 10000 + $2 * 100 + $3 }')
P=target/dx/placar/release/android/app
SO="$P/app/src/main/jniLibs/arm64-v8a/libmain.so"
mkdir -p dist "$TMPDIR"
MARK="$TMPDIR/android_release.mark"
touch "$MARK"

# 1. Interface em Rust e projeto Gradle. O dx também tenta compilar o
# servidor e falha (no Dioxus 0.7.10, mesmo com --fullstack false); o que
# interessa é a biblioteca do app, conferida logo abaixo.
echo "1/3 compilando (uns 10 minutos)..."
dx build --platform android --release --target aarch64-linux-android \
    --fullstack false --no-default-features --features mobile \
    > "$TMPDIR/android_release.log" 2>&1 || true
if [ ! -f "$SO" ] || [ "$SO" -ot "$MARK" ]; then
    echo "a compilação do app falhou; ver $TMPDIR/android_release.log" >&2
    exit 1
fi

# 2. O que o dx 0.7.10 não deixa configurar: ícone e número da versão.
echo "2/3 ícone e versão $VERSION ($CODE)..."
RES="$P/app/src/main/res"
for dir in icons/android/mipmap-*; do
    cp "$dir"/*.webp "$RES/$(basename "$dir")/"
done
cat > "$RES/drawable/ic_launcher_background.xml" <<'XML'
<?xml version="1.0" encoding="utf-8"?>
<shape xmlns:android="http://schemas.android.com/apk/res/android">
    <solid android:color="#1d6b4f" />
</shape>
XML
cat > "$RES/mipmap-anydpi-v26/ic_launcher.xml" <<'XML'
<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@drawable/ic_launcher_background" />
    <foreground android:drawable="@mipmap/ic_launcher_foreground" />
</adaptive-icon>
XML
sed -i '' "s/versionCode = [0-9]*/versionCode = $CODE/" "$P/app/build.gradle.kts"

# 3. Gradle, assinando com a chave de envio. A senha vem das Chaves do
# macOS e vai por variável de ambiente, sem aparecer na tela.
echo "3/3 empacotando e assinando..."
SENHA=$(security find-generic-password -s placar-upload-key -w)
(cd "$P" && env \
    "ORG_GRADLE_PROJECT_android.injected.signing.store.file=$HOME/.chaves/placar-upload.jks" \
    "ORG_GRADLE_PROJECT_android.injected.signing.store.password=$SENHA" \
    "ORG_GRADLE_PROJECT_android.injected.signing.key.alias=upload" \
    "ORG_GRADLE_PROJECT_android.injected.signing.key.password=$SENHA" \
    ./gradlew --quiet bundleRelease assembleRelease) >> "$TMPDIR/android_release.log" 2>&1 || {
    echo "o Gradle falhou; ver $TMPDIR/android_release.log" >&2
    exit 1
}
unset SENHA

cp "$P/app/build/outputs/bundle/release/app-release.aab" "dist/placar-$VERSION.aab"
cp "$P/app/build/outputs/apk/release/app-release.apk" "dist/placar-$VERSION.apk"
echo "pronto: dist/placar-$VERSION.aab (Play Store) e dist/placar-$VERSION.apk"
