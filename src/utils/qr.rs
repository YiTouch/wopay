// 二维码生成工具
// 提供支付二维码生成功能

use qrcode::QrCode;
use image::{ImageBuffer, Luma};
use base64;
use anyhow::{Result, Context};

/// 生成支付二维码
/// 
/// # Arguments
/// * `payment_url` - 支付链接 (如 ethereum:0x...?value=100)
/// 
/// # Returns
/// * Base64编码的PNG图片数据
pub fn generate_payment_qr_code(payment_url: &str) -> Result<String> {
    // 创建二维码
    let qr_code = QrCode::new(payment_url)
        .context("Failed to create QR code")?;

    // 渲染为图像
    let image = qr_code.render::<Luma<u8>>().build();
    
    // 转换为PNG格式的字节数组
    let mut png_data = Vec::new();
    {
        use image::codecs::png::PngEncoder;
        use image::ImageEncoder;
        
        let encoder = PngEncoder::new(&mut png_data);
        encoder.write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ColorType::L8,
        )
        .context("Failed to encode PNG")?;
    }

    // 转换为Base64
    let base64_data = base64::encode(&png_data);
    Ok(format!("data:image/png;base64,{}", base64_data))
}

/// 生成带Logo的二维码
/// 
/// # Arguments
/// * `payment_url` - 支付链接
/// * `logo_data` - Logo图片数据 (可选)
/// 
/// # Returns
/// * Base64编码的PNG图片数据
pub fn generate_qr_code_with_logo(payment_url: &str, logo_data: Option<&[u8]>) -> Result<String> {
    let qr_code = QrCode::new(payment_url)
        .context("Failed to create QR code")?;

    // 创建高分辨率的二维码图像
    let qr_image = qr_code.render::<Luma<u8>>()
        .min_dimensions(300, 300)
        .max_dimensions(300, 300)
        .build();

    let mut final_image = qr_image;

    // 如果提供了Logo，则添加到二维码中心
    if let Some(logo_bytes) = logo_data {
        if let Ok(logo_img) = image::load_from_memory(logo_bytes) {
            let logo_size = 60; // Logo大小
            let logo_resized = logo_img.resize(
                logo_size, 
                logo_size, 
                image::imageops::FilterType::Lanczos3
            );

            // 计算Logo位置 (居中)
            let qr_width = final_image.width();
            let qr_height = final_image.height();
            let logo_x = (qr_width - logo_size) / 2;
            let logo_y = (qr_height - logo_size) / 2;

            // 将Logo叠加到二维码上
            image::imageops::overlay(&mut final_image, &logo_resized, logo_x as i64, logo_y as i64);
        }
    }

    // 转换为PNG格式
    let mut png_data = Vec::new();
    {
        use image::codecs::png::PngEncoder;
        use image::ImageEncoder;
        
        let encoder = PngEncoder::new(&mut png_data);
        encoder.write_image(
            final_image.as_raw(),
            final_image.width(),
            final_image.height(),
            final_image.color(),
        )
        .context("Failed to encode PNG")?;
    }

    let base64_data = base64::encode(&png_data);
    Ok(format!("data:image/png;base64,{}", base64_data))
}

/// 验证二维码内容
/// 
/// # Arguments
/// * `content` - 二维码内容
/// 
/// # Returns
/// * 内容是否为有效的支付链接
pub fn validate_payment_qr_content(content: &str) -> bool {
    // 检查是否为有效的Ethereum支付链接
    if content.starts_with("ethereum:") {
        return validate_ethereum_payment_url(content);
    }
    
    // 检查是否为有效的比特币支付链接
    if content.starts_with("bitcoin:") {
        return validate_bitcoin_payment_url(content);
    }
    
    false
}

/// 验证Ethereum支付URL格式
fn validate_ethereum_payment_url(url: &str) -> bool {
    // 基础格式: ethereum:0x...?value=...
    if !url.starts_with("ethereum:0x") {
        return false;
    }
    
    // 提取地址部分
    let parts: Vec<&str> = url.split('?').collect();
    if parts.len() != 2 {
        return false;
    }
    
    let address_part = parts[0];
    let address = &address_part[9..]; // 去掉 "ethereum:" 前缀
    
    // 验证以太坊地址格式 (42字符，以0x开头)
    if address.len() != 42 || !address.starts_with("0x") {
        return false;
    }
    
    // 验证地址是否为有效的十六进制
    address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// 验证比特币支付URL格式 (预留功能)
fn validate_bitcoin_payment_url(_url: &str) -> bool {
    // TODO: 实现比特币地址验证
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_payment_qr_code() {
        let payment_url = "ethereum:0x742d35Cc6634C0532925a3b8D4C9db96DfbBb8b2?value=1000000000000000000";
        let qr_code = generate_payment_qr_code(payment_url).unwrap();
        
        assert!(qr_code.starts_with("data:image/png;base64,"));
        assert!(qr_code.len() > 100); // 确保有实际的图片数据
    }

    #[test]
    fn test_validate_ethereum_payment_url() {
        // 有效的Ethereum支付URL
        let valid_url = "ethereum:0x742d35Cc6634C0532925a3b8D4C9db96DfbBb8b2?value=1000000000000000000";
        assert!(validate_ethereum_payment_url(valid_url));
        
        // 无效的URL格式
        let invalid_urls = vec![
            "bitcoin:1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
            "ethereum:0x123", // 地址太短
            "ethereum:0xGGGG35Cc6634C0532925a3b8D4C9db96DfbBb8b2?value=100", // 包含非十六进制字符
            "0x742d35Cc6634C0532925a3b8D4C9db96DfbBb8b2", // 缺少协议前缀
        ];
        
        for url in invalid_urls {
            assert!(!validate_ethereum_payment_url(url), "URL should be invalid: {}", url);
        }
    }

    #[test]
    fn test_validate_payment_qr_content() {
        let valid_content = "ethereum:0x742d35Cc6634C0532925a3b8D4C9db96DfbBb8b2?value=1000000000000000000";
        assert!(validate_payment_qr_content(valid_content));
        
        let invalid_content = "https://example.com";
        assert!(!validate_payment_qr_content(invalid_content));
    }
}
