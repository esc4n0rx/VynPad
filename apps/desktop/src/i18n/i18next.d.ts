import type ptBR from "./pt-BR.json";

// Tipa as chaves de `t()`: uma chave que não existe em pt-BR.json não compila.
declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "common";
    resources: typeof ptBR;
  }
}
