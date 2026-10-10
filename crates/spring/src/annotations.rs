//! The Spring Framework annotations offered after an `@` in a Java or Kotlin
//! file, each with the import that makes it resolve.
//!
//! The table below is scoped to genuine `org.springframework.*`
//! packages only — verified against real jars on this machine (`spring-
//! context`/`spring-beans`/`spring-web`/`spring-tx`/`spring-boot-
//! autoconfigure`, several versions), not assumed. `@PostConstruct`/
//! `@PreDestroy` and similar JSR-250 annotations are deliberately excluded
//! even though they're routinely used in Spring code: their real package
//! is `javax.annotation.*` (pre-Spring-Boot-3) or `jakarta.annotation.*`
//! (Spring Boot 3+), and this codebase has no reliable signal yet for
//! which namespace a given project is on — inserting the wrong one would
//! be a real, silent correctness bug, not just an incomplete candidate
//! list, so these are left out rather than guessed at.

use fg_extension::ImportCandidate;

use crate::{JAVA, KOTLIN};

struct SpringAnnotation {
    name: &'static str,
    import_path: &'static str,
}

const SPRING_ANNOTATIONS: &[SpringAnnotation] = &[
    // org.springframework.stereotype
    SpringAnnotation {
        name: "Component",
        import_path: "org.springframework.stereotype.Component",
    },
    SpringAnnotation {
        name: "Controller",
        import_path: "org.springframework.stereotype.Controller",
    },
    SpringAnnotation {
        name: "Indexed",
        import_path: "org.springframework.stereotype.Indexed",
    },
    SpringAnnotation {
        name: "Repository",
        import_path: "org.springframework.stereotype.Repository",
    },
    SpringAnnotation {
        name: "Service",
        import_path: "org.springframework.stereotype.Service",
    },
    // org.springframework.beans.factory.annotation
    SpringAnnotation {
        name: "Autowired",
        import_path: "org.springframework.beans.factory.annotation.Autowired",
    },
    SpringAnnotation {
        name: "Configurable",
        import_path: "org.springframework.beans.factory.annotation.Configurable",
    },
    SpringAnnotation {
        name: "Lookup",
        import_path: "org.springframework.beans.factory.annotation.Lookup",
    },
    SpringAnnotation {
        name: "Qualifier",
        import_path: "org.springframework.beans.factory.annotation.Qualifier",
    },
    SpringAnnotation {
        name: "Required",
        import_path: "org.springframework.beans.factory.annotation.Required",
    },
    SpringAnnotation {
        name: "Value",
        import_path: "org.springframework.beans.factory.annotation.Value",
    },
    // org.springframework.context.annotation
    SpringAnnotation {
        name: "Bean",
        import_path: "org.springframework.context.annotation.Bean",
    },
    SpringAnnotation {
        name: "ComponentScan",
        import_path: "org.springframework.context.annotation.ComponentScan",
    },
    SpringAnnotation {
        name: "ComponentScans",
        import_path: "org.springframework.context.annotation.ComponentScans",
    },
    SpringAnnotation {
        name: "Conditional",
        import_path: "org.springframework.context.annotation.Conditional",
    },
    SpringAnnotation {
        name: "Configuration",
        import_path: "org.springframework.context.annotation.Configuration",
    },
    SpringAnnotation {
        name: "Description",
        import_path: "org.springframework.context.annotation.Description",
    },
    SpringAnnotation {
        name: "EnableAspectJAutoProxy",
        import_path: "org.springframework.context.annotation.EnableAspectJAutoProxy",
    },
    SpringAnnotation {
        name: "Import",
        import_path: "org.springframework.context.annotation.Import",
    },
    SpringAnnotation {
        name: "ImportResource",
        import_path: "org.springframework.context.annotation.ImportResource",
    },
    SpringAnnotation {
        name: "Lazy",
        import_path: "org.springframework.context.annotation.Lazy",
    },
    SpringAnnotation {
        name: "Primary",
        import_path: "org.springframework.context.annotation.Primary",
    },
    SpringAnnotation {
        name: "Profile",
        import_path: "org.springframework.context.annotation.Profile",
    },
    SpringAnnotation {
        name: "PropertySource",
        import_path: "org.springframework.context.annotation.PropertySource",
    },
    // org.springframework.transaction.annotation
    SpringAnnotation {
        name: "EnableTransactionManagement",
        import_path: "org.springframework.transaction.annotation.EnableTransactionManagement",
    },
    SpringAnnotation {
        name: "Transactional",
        import_path: "org.springframework.transaction.annotation.Transactional",
    },
    // org.springframework.boot.autoconfigure
    SpringAnnotation {
        name: "EnableAutoConfiguration",
        import_path: "org.springframework.boot.autoconfigure.EnableAutoConfiguration",
    },
    SpringAnnotation {
        name: "SpringBootApplication",
        import_path: "org.springframework.boot.autoconfigure.SpringBootApplication",
    },
    // org.springframework.web.bind.annotation
    SpringAnnotation {
        name: "ControllerAdvice",
        import_path: "org.springframework.web.bind.annotation.ControllerAdvice",
    },
    SpringAnnotation {
        name: "CookieValue",
        import_path: "org.springframework.web.bind.annotation.CookieValue",
    },
    SpringAnnotation {
        name: "CrossOrigin",
        import_path: "org.springframework.web.bind.annotation.CrossOrigin",
    },
    SpringAnnotation {
        name: "DeleteMapping",
        import_path: "org.springframework.web.bind.annotation.DeleteMapping",
    },
    SpringAnnotation {
        name: "ExceptionHandler",
        import_path: "org.springframework.web.bind.annotation.ExceptionHandler",
    },
    SpringAnnotation {
        name: "GetMapping",
        import_path: "org.springframework.web.bind.annotation.GetMapping",
    },
    SpringAnnotation {
        name: "InitBinder",
        import_path: "org.springframework.web.bind.annotation.InitBinder",
    },
    SpringAnnotation {
        name: "MatrixVariable",
        import_path: "org.springframework.web.bind.annotation.MatrixVariable",
    },
    SpringAnnotation {
        name: "ModelAttribute",
        import_path: "org.springframework.web.bind.annotation.ModelAttribute",
    },
    SpringAnnotation {
        name: "PatchMapping",
        import_path: "org.springframework.web.bind.annotation.PatchMapping",
    },
    SpringAnnotation {
        name: "PathVariable",
        import_path: "org.springframework.web.bind.annotation.PathVariable",
    },
    SpringAnnotation {
        name: "PostMapping",
        import_path: "org.springframework.web.bind.annotation.PostMapping",
    },
    SpringAnnotation {
        name: "PutMapping",
        import_path: "org.springframework.web.bind.annotation.PutMapping",
    },
    SpringAnnotation {
        name: "RequestAttribute",
        import_path: "org.springframework.web.bind.annotation.RequestAttribute",
    },
    SpringAnnotation {
        name: "RequestBody",
        import_path: "org.springframework.web.bind.annotation.RequestBody",
    },
    SpringAnnotation {
        name: "RequestHeader",
        import_path: "org.springframework.web.bind.annotation.RequestHeader",
    },
    SpringAnnotation {
        name: "RequestMapping",
        import_path: "org.springframework.web.bind.annotation.RequestMapping",
    },
    SpringAnnotation {
        name: "RequestParam",
        import_path: "org.springframework.web.bind.annotation.RequestParam",
    },
    SpringAnnotation {
        name: "RequestPart",
        import_path: "org.springframework.web.bind.annotation.RequestPart",
    },
    SpringAnnotation {
        name: "ResponseBody",
        import_path: "org.springframework.web.bind.annotation.ResponseBody",
    },
    SpringAnnotation {
        name: "ResponseStatus",
        import_path: "org.springframework.web.bind.annotation.ResponseStatus",
    },
    SpringAnnotation {
        name: "RestController",
        import_path: "org.springframework.web.bind.annotation.RestController",
    },
    SpringAnnotation {
        name: "RestControllerAdvice",
        import_path: "org.springframework.web.bind.annotation.RestControllerAdvice",
    },
    SpringAnnotation {
        name: "SessionAttribute",
        import_path: "org.springframework.web.bind.annotation.SessionAttribute",
    },
    SpringAnnotation {
        name: "SessionAttributes",
        import_path: "org.springframework.web.bind.annotation.SessionAttributes",
    },
];

/// Every known Spring annotation, unfiltered: the editor narrows the list by
/// what has been typed after the `@`. Empty for any language but the two JVM
/// ones.
pub fn candidates(language_id: &str) -> Vec<ImportCandidate> {
    if !matches!(language_id, JAVA | KOTLIN) {
        return Vec::new();
    }
    SPRING_ANNOTATIONS
        .iter()
        .map(|a| ImportCandidate {
            name: a.name.to_string(),
            qualified_name: a.import_path.to_string(),
        })
        .collect()
}

#[cfg(test)]
#[path = "annotations_test.rs"]
mod annotations_test;
