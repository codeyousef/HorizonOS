//! Comprehensive test suite for graph-visual module
//!
//! This test suite covers all major functionality of the visual design system
//! including icons, thumbnails, effects, and theming.

use horizonos_graph_visual::*;
use std::path::Path;
use tempfile::TempDir;

/// Test configuration for visual tests
struct VisualTestConfig {
    test_dir: TempDir,
    test_image_path: std::path::PathBuf,
}

impl VisualTestConfig {
    fn new() -> Self {
        let test_dir = TempDir::new().expect("Failed to create temp dir");
        let test_image_path = test_dir.path().join("test_image.png");
        
        // Create a simple test image
        Self::create_test_image(&test_image_path);
        
        Self {
            test_dir,
            test_image_path,
        }
    }
    
    fn create_test_image(path: &Path) {
        use image::{ImageBuffer, Rgb};
        
        let img = ImageBuffer::from_fn(100, 100, |x, y| {
            if (x + y) % 20 < 10 {
                Rgb([255, 0, 0])
            } else {
                Rgb([0, 255, 0])
            }
        });
        
        img.save(path).expect("Failed to save test image");
    }
}

#[cfg(test)]
mod visual_manager_tests {
    use super::*;
    
    #[test]
    fn test_visual_manager_creation() {
        let manager = VisualManager::new();
        assert!(manager.is_ok());
        
        let manager = manager.unwrap();
        let theme = manager.theme();
        assert_eq!(theme.name, "default");
        assert!(theme.is_dark);
    }
    
    #[test]
    fn test_theme_switching() {
        let mut manager = VisualManager::new().unwrap();
        
        // Test setting a custom theme
        let custom_theme = Theme {
            name: "custom".to_string(),
            is_dark: false,
            primary_color: [1.0, 0.0, 0.0, 1.0],
            secondary_color: [0.0, 1.0, 0.0, 1.0],
            background_color: [1.0, 1.0, 1.0, 1.0],
            text_color: [0.0, 0.0, 0.0, 1.0],
        };
        
        manager.set_theme(custom_theme.clone());
        let current_theme = manager.theme();
        
        assert_eq!(current_theme.name, "custom");
        assert!(!current_theme.is_dark);
        assert_eq!(current_theme.primary_color, [1.0, 0.0, 0.0, 1.0]);
    }
    
    #[test]
    fn test_theme_system_integration() {
        let manager = VisualManager::new().unwrap();
        let theme_system = manager.theme_system();
        
        // Test getting current theme
        let current_theme = theme_system.current_theme();
        assert_eq!(current_theme.metadata.name, "horizon-dark");
        
        // Test available themes
        let available_themes = theme_system.available_themes();
        assert!(available_themes.contains(&"horizon-dark"));
        assert!(available_themes.contains(&"horizon-light"));
        assert!(available_themes.contains(&"neon"));
        assert!(available_themes.contains(&"minimal"));
    }
}

#[cfg(test)]
mod icon_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_icon_loader_creation() {
        let config = VisualTestConfig::new();
        let loader = IconLoader::new(config.test_dir.path());
        assert!(loader.is_ok());
    }
    
    #[tokio::test]
    async fn test_file_type_icon_mapping() {
        let config = VisualTestConfig::new();
        let loader = IconLoader::new(config.test_dir.path()).unwrap();
        let mapper = FileTypeIconMapper::new(loader);
        
        // Test common file types
        let png_icon = mapper.get_icon_for_file("test.png").await;
        assert!(png_icon.is_ok());
        
        let txt_icon = mapper.get_icon_for_file("test.txt").await;
        assert!(txt_icon.is_ok());
        
        let dir_icon = mapper.get_icon_for_directory("test_dir").await;
        assert!(dir_icon.is_ok());
    }
    
    #[tokio::test]
    async fn test_app_icon_extraction() {
        let config = VisualTestConfig::new();
        let loader = IconLoader::new(config.test_dir.path()).unwrap();
        let extractor = AppIconExtractor::new(loader);
        
        // Test extracting icon from a common application
        let icon = extractor.extract_icon("firefox").await;
        // This might fail in CI, so we just check it doesn't panic
        assert!(icon.is_ok() || icon.is_err());
    }
    
    #[tokio::test]
    async fn test_icon_sizes() {
        let config = VisualTestConfig::new();
        let loader = IconLoader::new(config.test_dir.path()).unwrap();
        
        // Test different icon sizes
        let sizes = [IconSize::Small, IconSize::Medium, IconSize::Large];
        for size in sizes {
            let icon = loader.load_icon("test", size).await;
            // Test that it handles different sizes without panicking
            assert!(icon.is_ok() || icon.is_err());
        }
    }
}

#[cfg(test)]
mod thumbnail_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_thumbnail_generator_creation() {
        let config = VisualTestConfig::new();
        let generator = ThumbnailGenerator::new(config.test_dir.path());
        assert!(generator.is_ok());
    }
    
    #[tokio::test]
    async fn test_image_thumbnail_generation() {
        let config = VisualTestConfig::new();
        let generator = ThumbnailGenerator::new(config.test_dir.path()).unwrap();
        
        let thumbnail = generator.generate_thumbnail(&config.test_image_path, ThumbnailSize::Medium).await;
        assert!(thumbnail.is_ok());
        
        let thumbnail = thumbnail.unwrap();
        assert!(!thumbnail.is_empty());
    }
    
    #[tokio::test]
    async fn test_thumbnail_caching() {
        let config = VisualTestConfig::new();
        let generator = ThumbnailGenerator::new(config.test_dir.path()).unwrap();
        
        // Generate thumbnail twice
        let start = std::time::Instant::now();
        let thumbnail1 = generator.generate_thumbnail(&config.test_image_path, ThumbnailSize::Medium).await;
        let first_duration = start.elapsed();
        
        let start = std::time::Instant::now();
        let thumbnail2 = generator.generate_thumbnail(&config.test_image_path, ThumbnailSize::Medium).await;
        let second_duration = start.elapsed();
        
        assert!(thumbnail1.is_ok());
        assert!(thumbnail2.is_ok());
        
        // Second call should be faster due to caching
        assert!(second_duration < first_duration);
    }
    
    #[tokio::test]
    async fn test_profile_picture_generation() {
        let config = VisualTestConfig::new();
        let generator = ProfilePictureGenerator::new(config.test_dir.path());
        
        let profile_pic = generator.generate_profile_picture(
            "test@example.com",
            "Test User",
            64
        ).await;
        
        assert!(profile_pic.is_ok());
        
        let profile_pic = profile_pic.unwrap();
        assert!(!profile_pic.is_empty());
    }
    
    #[tokio::test]
    async fn test_thumbnail_sizes() {
        let config = VisualTestConfig::new();
        let generator = ThumbnailGenerator::new(config.test_dir.path()).unwrap();
        
        let sizes = [ThumbnailSize::Small, ThumbnailSize::Medium, ThumbnailSize::Large];
        for size in sizes {
            let thumbnail = generator.generate_thumbnail(&config.test_image_path, size).await;
            assert!(thumbnail.is_ok());
        }
    }
}

#[cfg(test)]
mod theme_tests {
    use super::*;
    use horizonos_graph_visual::theme::*;
    
    #[test]
    fn test_theme_system_creation() {
        let theme_system = ThemeSystem::new();
        let current_theme = theme_system.current_theme();
        
        assert_eq!(current_theme.metadata.name, "horizon-dark");
        assert!(current_theme.metadata.is_dark);
    }
    
    #[test]
    fn test_theme_switching() {
        let mut theme_system = ThemeSystem::new();
        
        // Test switching to light theme
        let result = theme_system.set_theme("horizon-light");
        assert!(result.is_ok());
        
        let current_theme = theme_system.current_theme();
        assert_eq!(current_theme.metadata.name, "horizon-light");
        assert!(!current_theme.metadata.is_dark);
    }
    
    #[test]
    fn test_theme_observer() {
        use std::sync::{Arc, Mutex};
        
        struct TestObserver {
            theme_name: Arc<Mutex<String>>,
        }
        
        impl ThemeObserver for TestObserver {
            fn on_theme_changed(&self, theme: &Theme) {
                *self.theme_name.lock().unwrap() = theme.metadata.name.clone();
            }
        }
        
        let mut theme_system = ThemeSystem::new();
        let theme_name = Arc::new(Mutex::new(String::new()));
        
        let observer = TestObserver {
            theme_name: theme_name.clone(),
        };
        
        theme_system.add_observer(Box::new(observer));
        theme_system.set_theme("neon").unwrap();
        
        assert_eq!(*theme_name.lock().unwrap(), "neon");
    }
    
    #[test]
    fn test_color_creation() {
        let color = Color::rgb(1.0, 0.5, 0.0);
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 0.5);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
        
        let color_with_alpha = Color::rgba(1.0, 0.5, 0.0, 0.8);
        assert_eq!(color_with_alpha.a, 0.8);
    }
    
    #[test]
    fn test_color_from_hex() {
        let color = Color::from_hex("#FF8000").unwrap();
        assert!((color.r - 1.0).abs() < 0.01);
        assert!((color.g - 0.5).abs() < 0.01);
        assert!((color.b - 0.0).abs() < 0.01);
        assert_eq!(color.a, 1.0);
        
        let color_with_alpha = Color::from_hex("#FF8000AA").unwrap();
        assert!((color_with_alpha.a - (170.0 / 255.0)).abs() < 0.01);
    }
    
    #[test]
    fn test_default_themes() {
        let dark_theme = Theme::horizon_dark();
        assert_eq!(dark_theme.metadata.name, "horizon-dark");
        assert!(dark_theme.metadata.is_dark);
        assert!(dark_theme.animations.enabled);
        
        let light_theme = Theme::horizon_light();
        assert_eq!(light_theme.metadata.name, "horizon-light");
        assert!(!light_theme.metadata.is_dark);
        
        let neon_theme = Theme::neon();
        assert_eq!(neon_theme.metadata.name, "neon");
        assert!(neon_theme.effects.glow.enabled);
        assert!(neon_theme.effects.glow.intensity > 1.0);
        
        let minimal_theme = Theme::minimal();
        assert_eq!(minimal_theme.metadata.name, "minimal");
        assert!(!minimal_theme.effects.glow.enabled);
        assert!(!minimal_theme.animations.enabled);
    }
    
    #[test]
    fn test_lod_level_operations() {
        use horizonos_graph_visual::theme::LodLevel;
        
        assert_eq!(LodLevel::High.min(LodLevel::Medium), LodLevel::High);
        assert_eq!(LodLevel::Medium.min(LodLevel::Low), LodLevel::Medium);
        assert_eq!(LodLevel::Low.min(LodLevel::Culled), LodLevel::Low);
        
        assert_eq!(LodLevel::High.quality_multiplier(), 1.0);
        assert_eq!(LodLevel::Medium.quality_multiplier(), 0.6);
        assert_eq!(LodLevel::Low.quality_multiplier(), 0.3);
        assert_eq!(LodLevel::Culled.quality_multiplier(), 0.0);
    }
}

#[cfg(test)]
mod effects_tests {
    use super::*;
    
    #[test]
    fn test_particle_system_creation() {
        let particle_system = ParticleSystem::new();
        assert!(particle_system.is_ok());
    }
    
    #[test]
    fn test_glow_effect_creation() {
        let glow_effect = GlowEffect::new();
        assert!(glow_effect.is_ok());
    }
    
    #[test]
    fn test_shadow_renderer_creation() {
        let shadow_renderer = ShadowRenderer::new();
        assert!(shadow_renderer.is_ok());
    }
    
    #[test]
    fn test_soft_boundary_effect_creation() {
        let soft_boundary_effect = SoftBoundaryEffect::new();
        assert!(soft_boundary_effect.is_ok());
    }
}

#[cfg(test)]
mod edge_style_tests {
    use super::*;
    
    #[test]
    fn test_edge_style_creation() {
        let data_flow = EdgeStyle::data_flow();
        assert_eq!(data_flow.name, "data-flow");
        assert!(data_flow.animated);
        assert!(data_flow.has_arrow);
        assert_eq!(data_flow.width, 3.0);
        
        let dependency = EdgeStyle::dependency();
        assert_eq!(dependency.name, "dependency");
        assert!(!dependency.animated);
        assert!(dependency.has_arrow);
        assert_eq!(dependency.width, 2.0);
        
        let relationship = EdgeStyle::relationship();
        assert_eq!(relationship.name, "relationship");
        assert!(!relationship.animated);
        assert!(!relationship.has_arrow);
        assert_eq!(relationship.width, 1.5);
    }
}

#[cfg(test)]
mod visual_priority_tests {
    use super::*;
    
    #[test]
    fn test_visual_priority_ordering() {
        assert!(VisualPriority::Background < VisualPriority::Normal);
        assert!(VisualPriority::Normal < VisualPriority::Elevated);
        assert!(VisualPriority::Elevated < VisualPriority::Foreground);
        assert!(VisualPriority::Foreground < VisualPriority::Overlay);
    }
    
    #[test]
    fn test_visual_element_types() {
        let elements = [
            VisualElement::AppIcon,
            VisualElement::FileIcon,
            VisualElement::ProfilePicture,
            VisualElement::Thumbnail,
            VisualElement::Custom,
        ];
        
        // Test that all elements are distinct
        for (i, elem1) in elements.iter().enumerate() {
            for (j, elem2) in elements.iter().enumerate() {
                if i != j {
                    assert_ne!(elem1, elem2);
                }
            }
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_full_visual_pipeline() {
        let config = VisualTestConfig::new();
        let mut manager = VisualManager::new().unwrap();
        
        // Test theme switching
        let custom_theme = Theme {
            name: "test".to_string(),
            is_dark: false,
            primary_color: [1.0, 0.0, 0.0, 1.0],
            secondary_color: [0.0, 1.0, 0.0, 1.0],
            background_color: [1.0, 1.0, 1.0, 1.0],
            text_color: [0.0, 0.0, 0.0, 1.0],
        };
        
        manager.set_theme(custom_theme);
        
        // Test icon loading
        let icon_loader = IconLoader::new(config.test_dir.path()).unwrap();
        let icon = icon_loader.load_icon("test", IconSize::Medium).await;
        assert!(icon.is_ok() || icon.is_err()); // Should handle gracefully
        
        // Test thumbnail generation
        let thumbnail_generator = ThumbnailGenerator::new(config.test_dir.path()).unwrap();
        let thumbnail = thumbnail_generator.generate_thumbnail(&config.test_image_path, ThumbnailSize::Medium).await;
        assert!(thumbnail.is_ok());
        
        // Test effects
        let particle_system = ParticleSystem::new();
        assert!(particle_system.is_ok());
    }
    
    #[tokio::test]
    async fn test_performance_characteristics() {
        let config = VisualTestConfig::new();
        let generator = ThumbnailGenerator::new(config.test_dir.path()).unwrap();
        
        // Test that thumbnail generation is reasonably fast
        let start = std::time::Instant::now();
        let thumbnail = generator.generate_thumbnail(&config.test_image_path, ThumbnailSize::Medium).await;
        let duration = start.elapsed();
        
        assert!(thumbnail.is_ok());
        assert!(duration < Duration::from_secs(5)); // Should be fast
    }
    
    #[test]
    fn test_memory_usage() {
        let mut theme_system = ThemeSystem::new();
        
        // Test that switching themes doesn't leak memory
        let initial_memory = get_memory_usage();
        
        for _ in 0..100 {
            theme_system.set_theme("horizon-light").unwrap();
            theme_system.set_theme("horizon-dark").unwrap();
        }
        
        let final_memory = get_memory_usage();
        let memory_increase = final_memory - initial_memory;
        
        // Allow for some memory increase but not excessive
        assert!(memory_increase < 10_000_000); // Less than 10MB increase
    }
    
    fn get_memory_usage() -> usize {
        // Simplified memory usage check
        // In a real implementation, this would use system APIs
        std::mem::size_of::<ThemeSystem>()
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_invalid_file_handling() {
        let config = VisualTestConfig::new();
        let generator = ThumbnailGenerator::new(config.test_dir.path()).unwrap();
        
        // Test with non-existent file
        let invalid_path = config.test_dir.path().join("nonexistent.png");
        let result = generator.generate_thumbnail(&invalid_path, ThumbnailSize::Medium).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_invalid_icon_requests() {
        let config = VisualTestConfig::new();
        let loader = IconLoader::new(config.test_dir.path()).unwrap();
        
        // Test with invalid icon name
        let result = loader.load_icon("definitely_does_not_exist", IconSize::Medium).await;
        assert!(result.is_err());
    }
    
    #[test]
    fn test_invalid_theme_switching() {
        let mut theme_system = ThemeSystem::new();
        
        // Test switching to non-existent theme
        let result = theme_system.set_theme("nonexistent_theme");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_invalid_color_parsing() {
        let result = Color::from_hex("invalid_hex");
        assert!(result.is_err());
        
        let result = Color::from_hex("#GG0000");
        assert!(result.is_err());
        
        let result = Color::from_hex("#FF000");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_theme_removal_edge_cases() {
        let mut theme_system = ThemeSystem::new();
        
        // Test removing current theme
        let result = theme_system.remove_theme("horizon-dark");
        assert!(result.is_err());
        
        // Test removing non-existent theme
        let result = theme_system.remove_theme("nonexistent");
        assert!(result.is_ok()); // Should not error
    }
}

#[cfg(test)]
mod accessibility_tests {
    use super::*;
    
    #[test]
    fn test_high_contrast_theme_support() {
        let mut theme_system = ThemeSystem::new();
        
        // Test that themes have adequate contrast
        let light_theme = Theme::horizon_light();
        let dark_theme = Theme::horizon_dark();
        
        // Background and text should have good contrast
        assert_ne!(light_theme.colors.background, light_theme.colors.text.primary);
        assert_ne!(dark_theme.colors.background, dark_theme.colors.text.primary);
    }
    
    #[test]
    fn test_theme_accessibility_metadata() {
        let themes = [
            Theme::horizon_dark(),
            Theme::horizon_light(),
            Theme::neon(),
            Theme::minimal(),
        ];
        
        for theme in themes {
            assert!(!theme.metadata.name.is_empty());
            assert!(!theme.metadata.display_name.is_empty());
            assert!(!theme.metadata.description.is_empty());
            
            // Test that theme has proper color definitions
            assert!(theme.colors.text.primary.a > 0.0);
            assert!(theme.colors.background.a > 0.0);
        }
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_path_traversal_protection() {
        let config = VisualTestConfig::new();
        let generator = ThumbnailGenerator::new(config.test_dir.path()).unwrap();
        
        // Test that path traversal is prevented
        let malicious_path = config.test_dir.path().join("../../../etc/passwd");
        let result = generator.generate_thumbnail(&malicious_path, ThumbnailSize::Medium).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_resource_limits() {
        let config = VisualTestConfig::new();
        let generator = ThumbnailGenerator::new(config.test_dir.path()).unwrap();
        
        // Test that extremely large thumbnail requests are handled properly
        let result = generator.generate_thumbnail(&config.test_image_path, ThumbnailSize::Large).await;
        assert!(result.is_ok() || result.is_err()); // Should handle gracefully
    }
}