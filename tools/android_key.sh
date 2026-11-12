#!/bin/sh
# Chave de envio da Play Store (upload key). Rodar no Terminal do Mac.
#
#   sh tools/android_key.sh criar      cria a chave e guarda a senha nas Chaves
#   sh tools/android_key.sh conferir   diz se a chave e a senha estão no lugar
#
# A chave fica em ~/.chaves/placar-upload.jks e a senha nas Chaves do macOS
# (item "placar-upload-key"); a senha nunca aparece na tela. A chave que
# assina o app para os celulares fica com o Google: perder esta só obriga a
# pedir a troca no Play Console.
#
# Trocar de Mac: o Assistente de Migração leva as duas. Sem ele, copiar o
# .jks para ~/.chaves/ e guardar a senha de novo:
#   security add-generic-password -a placar -s placar-upload-key -w 'A_SENHA'
# (a senha sai com: security find-generic-password -s placar-upload-key -w)
set -eu

A=/Volumes/KINGSTON/Android
KEYSTORE="$HOME/.chaves/placar-upload.jks"
ITEM=placar-upload-key

conferir() {
    if [ -f "$KEYSTORE" ] && security find-generic-password -s "$ITEM" >/dev/null 2>&1; then
        echo ok
    else
        echo "falta a chave ($KEYSTORE) ou a senha nas Chaves ($ITEM)" >&2
        exit 1
    fi
}

criar() {
    if [ -e "$KEYSTORE" ] || security find-generic-password -s "$ITEM" >/dev/null 2>&1; then
        echo "já existe uma chave ou senha; nada foi mudado" >&2
        exit 1
    fi
    mkdir -p "$HOME/.chaves" && chmod 700 "$HOME/.chaves"
    SENHA=$(openssl rand -base64 24 | tr -d '/+=')
    security add-generic-password -a placar -s "$ITEM" -w "$SENHA"
    "$A/jdk/Contents/Home/bin/keytool" -genkeypair -keystore "$KEYSTORE" \
        -alias upload -keyalg RSA -keysize 2048 -validity 10000 \
        -storepass "$SENHA" -keypass "$SENHA" \
        -dname "CN=Santa Maria Software, O=Santa Maria Software, C=BR"
    unset SENHA
    conferir
}

case "${1:-}" in
    criar) criar ;;
    conferir) conferir ;;
    *) sed -n '2,5p' "$0" | sed 's/^# \{0,1\}//'; exit 1 ;;
esac
