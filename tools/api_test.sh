#!/bin/bash
# Teste de ponta a ponta da API do placar.
B=http://127.0.0.1:8080
D=$(dirname "$0")
J=$D/jar.txt; J2=$D/jar2.txt; rm -f $J $J2
U="andre$RANDOM"; U2="pai$RANDOM"
post() { curl -s -b $1 -c $1 -X POST -H 'Content-Type: application/json' "$B$2" -d "$3"; echo; }

echo "== cadastro"; post $J /api/cadastro "{\"username\":\"$U\",\"display_name\":\"André\",\"password\":\"segredo1\"}"
echo "== me"; curl -s -b $J $B/api/me; echo
echo "== senha errada"; post $J2 /api/entrar "{\"username\":\"$U\",\"password\":\"errada\"}"
echo "== cria sala individual"
CODE=$(post $J /api/salas "{\"name\":\"Teste\",\"mode\":\"Individual\",\"players\":[{\"id\":null,\"name\":\"André\",\"username\":\"$U\"},{\"id\":null,\"name\":\"Pai\",\"username\":null}]}" | tr -d '"\n')
echo "codigo: $CODE"

# Fica escutando o SSE enquanto as mãos são registradas.
curl -s -N --max-time 6 $B/api/salas/$CODE/eventos > $D/sse.txt &
sleep 1

echo "== começa boi"; post $J /api/salas/$CODE/boi '{}'
echo "== sem quem saiu certo (assento 5)"; post $J /api/salas/$CODE/maos '{"expected_hands":0,"starter_seat":5,"outcome":"Batida","winner_side":0}'
# Pai faz 2, empate, André vira com 4 seguidas: lambreta reversa.
n=0
for w in 1 1 null 0 0 0 0; do
  o=Batida; [ "$w" = null ] && o=Empate
  post $J /api/salas/$CODE/maos "{\"expected_hands\":$n,\"starter_seat\":$((n%2)),\"outcome\":\"$o\",\"winner_side\":$w}"
  n=$((n+1))
done
echo "== mão repetida (expected desatualizado)"; post $J /api/salas/$CODE/maos '{"expected_hands":3,"starter_seat":0,"outcome":"Batida","winner_side":0}'
echo "== mão depois do fim"; post $J /api/salas/$CODE/maos "{\"expected_hands\":$n,\"starter_seat\":0,\"outcome\":\"Batida\",\"winner_side\":0}"
echo "== visitante não marca"; post $J2 /api/salas/$CODE/maos "{\"expected_hands\":$n,\"starter_seat\":0,\"outcome\":\"Batida\",\"winner_side\":0}"
echo "== visitante vê a sala"; curl -s $B/api/salas/$CODE | head -c 600; echo
echo "== desfaz e refaz a última"; post $J /api/salas/$CODE/desfazer "{\"expected_hands\":$n}"
post $J /api/salas/$CODE/maos "{\"expected_hands\":$((n-1)),\"starter_seat\":0,\"outcome\":\"Fechado\",\"winner_side\":0}"
echo "== edita jogadores (vincula pai a conta nova)"
post $J2 /api/cadastro "{\"username\":\"$U2\",\"display_name\":\"Pai\",\"password\":\"segredo2\"}" >/dev/null
ROOM=$(curl -s $B/api/salas/$CODE)
P2=$(echo "$ROOM" | python3 -c 'import json,sys;print(json.load(sys.stdin)["players"][1]["id"])')
P1=$(echo "$ROOM" | python3 -c 'import json,sys;print(json.load(sys.stdin)["players"][0]["id"])')
post $J /api/salas/$CODE/jogadores "{\"players\":[{\"id\":$P2,\"name\":\"Pai\",\"username\":\"$U2\"},{\"id\":$P1,\"name\":\"André\",\"username\":\"$U\"},{\"id\":null,\"name\":\"Sobrinho\",\"username\":null}]}"
echo "== pai agora pode marcar? começa boi 2"; post $J2 /api/salas/$CODE/boi '{}'
echo "== minhas salas (pai)"; curl -s -b $J2 $B/api/minhas-salas; echo
echo "== estatísticas do pai"; curl -s -b $J2 $B/api/minhas-estatisticas; echo
echo "== sala final"; curl -s $B/api/salas/$CODE | python3 -m json.tool | head -60
echo "== sair"; post $J /api/sair '{}'; curl -s -b $J $B/api/me; echo
wait
echo "== eventos SSE recebidos: $(grep -c '^data' $D/sse.txt)"
