//! Invólucro para valores sensíveis (chaves, tokens, PIN) que nunca devem aparecer em logs.

use std::fmt;

/// Guarda um valor sensível. `Debug` e `Display` sempre imprimem `Secret(***)`,
/// então o valor não vaza por `tracing`, `{:?}` ou mensagens de erro.
///
/// O acesso ao conteúdo é explícito, via [`Secret::expose`].
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// Acesso explícito ao valor. Use só onde o segredo é realmente necessário.
    pub const fn expose(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for Secret<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_e_display_mascaram_o_valor() {
        let secret = Secret::new("token-super-secreto");
        assert_eq!(format!("{secret:?}"), "Secret(***)");
        assert_eq!(format!("{secret}"), "Secret(***)");
    }

    #[test]
    fn nao_vaza_dentro_de_structs_com_debug_derivado() {
        #[derive(Debug)]
        #[allow(dead_code)]
        struct Credenciais {
            usuario: &'static str,
            token: Secret<String>,
        }
        let c = Credenciais {
            usuario: "celular",
            token: Secret::new("abc123".into()),
        };
        let texto = format!("{c:?}");
        assert!(!texto.contains("abc123"));
        assert!(texto.contains("Secret(***)"));
    }

    #[test]
    fn expose_devolve_o_valor() {
        let secret = Secret::new([7u8; 4]);
        assert_eq!(secret.expose(), &[7u8; 4]);
        assert_eq!(secret.into_inner(), [7u8; 4]);
    }
}
