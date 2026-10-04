//! QR code da URI de pareamento, como SVG para a UI.

use qrcode::QrCode;
use qrcode::render::svg;

/// Erro ao gerar o QR (texto grande demais para um QR code).
#[derive(Debug, thiserror::Error)]
#[error("não foi possível gerar o QR code: {0}")]
pub struct QrError(#[from] qrcode::types::QrError);

/// Renderiza `uri` como SVG (módulos escuros sobre fundo branco, com margem).
///
/// O fundo é sempre branco: o contraste é o que garante a leitura pela câmera, mesmo no tema escuro.
pub fn pairing_qr_svg(uri: &str) -> Result<String, QrError> {
    let code = QrCode::new(uri.as_bytes())?;
    Ok(code
        .render::<svg::Color<'_>>()
        .min_dimensions(256, 256)
        .quiet_zone(true)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_svg_for_a_typical_uri() {
        let uri = "vynpad://pair?v=1&h=192.168.0.10&p=47800&fp=RO_FLCbhKpJLrtZQNQz-GQyRPVwe1JcK7inJGAK9fzA&t=EBESExQVFhcYGRobHB0eHw&n=PC-Sala";
        let svg = pairing_qr_svg(uri).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("#ffffff"));
    }

    #[test]
    fn oversized_input_is_an_error() {
        assert!(pairing_qr_svg(&"x".repeat(10_000)).is_err());
    }
}
