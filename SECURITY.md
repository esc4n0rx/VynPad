# Política de segurança

O VynPad aceita conexões da rede local e cria dispositivos de entrada no PC, então a segurança é
parte do produto. O modelo de ameaças e as defesas estão em [docs/SEGURANCA.md](docs/SEGURANCA.md).

## Como reportar uma vulnerabilidade

**Não abra uma issue pública** com detalhes de uma vulnerabilidade.

1. Abra a aba **Security** do repositório e use **Report a vulnerability** (relato privado).
2. Se esse botão não estiver disponível, abra uma issue pedindo um canal de contato privado,
   **sem** descrever o problema.

Inclua, se puder: a versão do VynPad (Configurações → Sobre), o sistema operacional, os passos para
reproduzir e o impacto que você enxerga. **Não inclua** tokens, chaves ou PINs reais; o pacote de
diagnóstico do app já os omite.

## O que esperar

- Confirmação do recebimento em poucos dias.
- Uma avaliação e, se confirmada, a correção em um release; você será avisado antes da divulgação.
- Pedimos que a divulgação pública espere a correção estar disponível.

## Versões com correções de segurança

Apenas a versão mais recente recebe correções. O app avisa quando há atualização
(Configurações → Sobre).

## Dentro e fora do escopo

Dentro: autenticação e pareamento, o canal TLS, o protocolo UDP, o instalador e a atualização
automática, o tratamento de arquivos de perfil e o uso indevido do driver virtual.

Fora (riscos conhecidos e aceitos, ver a seção 5 de [docs/SEGURANCA.md](docs/SEGURANCA.md)):
um atacante que já controla o seu PC ou o seu roteador por completo, e vulnerabilidades do
ViGEmBus em si (reporte ao [projeto original](https://github.com/nefarius/ViGEmBus)).
