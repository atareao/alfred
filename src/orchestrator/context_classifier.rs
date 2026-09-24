#[derive(Debug, Clone, PartialEq)]
pub enum ContextStrategy {
    SlidingWindow,
    Historical,
    RAG,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Override {
    None,
    Historical(String),
    Doc(String),
    Reset,
}

pub struct Classification {
    pub strategy: ContextStrategy,
    pub override_cmd: Override,
    pub confidence: f32,
}

pub struct ContextClassifier;

impl Default for ContextClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextClassifier {
    pub fn new() -> Self {
        Self
    }

    pub fn classify(&self, message: &str) -> Classification {
        let override_cmd = self.check_override(message);
        let confidence: f32 = if !matches!(&override_cmd, Override::None) {
            1.0
        } else {
            0.7
        };
        let strategy = match &override_cmd {
            Override::Historical(_) => ContextStrategy::Historical,
            Override::Doc(_) => ContextStrategy::RAG,
            Override::Reset | Override::None => ContextStrategy::SlidingWindow,
        };
        Classification {
            strategy,
            override_cmd,
            confidence,
        }
    }

    pub fn check_override(&self, message: &str) -> Override {
        let trimmed = message.trim();
        if let Some(query) = trimmed.strip_prefix("!historico ") {
            Override::Historical(query.to_string())
        } else if let Some(query) = trimmed.strip_prefix("!doc ") {
            Override::Doc(query.to_string())
        } else if trimmed == "!reset" {
            Override::Reset
        } else {
            Override::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_classification() {
        let classifier = ContextClassifier::new();
        let result = classifier.classify("Añade leche a la compra");
        assert_eq!(result.strategy, ContextStrategy::SlidingWindow);
        assert_eq!(result.override_cmd, Override::None);
    }

    #[test]
    fn test_historico_override() {
        let classifier = ContextClassifier::new();
        let result = classifier.check_override("!historico ¿qué planes hicimos?");
        assert!(matches!(result, Override::Historical(q) if q == "¿qué planes hicimos?"));
    }

    #[test]
    fn test_doc_override() {
        let classifier = ContextClassifier::new();
        let result = classifier.check_override("!doc recomendación hotel");
        assert!(matches!(result, Override::Doc(q) if q == "recomendación hotel"));
    }

    #[test]
    fn test_reset_override() {
        let classifier = ContextClassifier::new();
        let result = classifier.check_override("!reset");
        assert_eq!(result, Override::Reset);
    }

    #[test]
    fn test_no_override() {
        let classifier = ContextClassifier::new();
        let result = classifier.check_override("Hola, ¿cómo estás?");
        assert_eq!(result, Override::None);
    }

    #[test]
    fn test_classify_with_historico() {
        let classifier = ContextClassifier::new();
        let result = classifier.classify("!historico ¿qué pasó ayer?");
        assert_eq!(result.strategy, ContextStrategy::Historical);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn test_classify_with_doc() {
        let classifier = ContextClassifier::new();
        let result = classifier.classify("!doc receta pasta");
        assert_eq!(result.strategy, ContextStrategy::RAG);
    }

    #[test]
    fn test_classify_with_reset() {
        let classifier = ContextClassifier::new();
        let result = classifier.classify("!reset");
        assert_eq!(result.strategy, ContextStrategy::SlidingWindow);
    }
}
