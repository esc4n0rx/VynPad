# T-908 — Gyro / acelerômetro

| Campo | Valor |
|---|---|
| Fase | F9 — Client Android |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-906 |
| Status | ✅ Concluída (validada no aparelho) |

## Objetivo
Mira por movimento: o celular inclina e o analógico se move (a lógica de mapeamento fica no server, conforme D-08).

## Escopo
- Ativar sensores só quando o server envia `config{gyro_enabled: true}`.
- `SensorManager` com `TYPE_GYROSCOPE` + `TYPE_ACCELEROMETER` em `SENSOR_DELAY_FASTEST`, numa thread de handler própria.
- Compensar a orientação landscape (rotação dos eixos conforme `display.rotation`).
- Enviar o pacote `0x02` com os valores mais recentes na mesma cadência do input (quando o gyro está ativo, envia a ≥ 125 Hz mesmo sem toque).
- Calibração: "segure o celular parado por 2 s" para estimar o bias do gyro; salvo por aparelho.
- Botão de recentralizar no menu de pausa.

## Critérios de aceite
- [x] Com o perfil "gyro no stick direito", a mira num FPS responde de forma suave e sem drift perceptível em 60 s.
- [x] Desligar o gyro no server desliga os sensores (economia de bateria).

## Fora de escopo
- Emular motion do DualShock (fora do escopo X360).
