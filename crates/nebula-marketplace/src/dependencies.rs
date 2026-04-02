//! Skill dependency resolution system.
//!
//! This module provides a comprehensive dependency resolution system for skills,
//! including circular dependency detection, version conflict resolution,
//! optional dependency handling, and transitive dependency resolution.

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

/// Represents a dependency that a skill requires.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dependency {
    /// The name of the dependency (e.g., "logging", "database").
    pub name: String,
    /// The version requirement (e.g., ">=1.0.0", "^2.0").
    pub version_req: VersionReq,
    /// Whether this dependency is optional.
    pub optional: bool,
    /// Features to enable for this dependency.
    pub features: Vec<String>,
}

impl Dependency {
    /// Creates a new required dependency with the given name and version requirement.
    pub fn new(name: impl Into<String>, version_req: impl Into<String>) -> Result<Self, DependencyError> {
        let name_str = name.into();
        let version_str = version_req.into();
        let version_req = VersionReq::parse(&version_str)
            .map_err(|e| DependencyError::InvalidVersion { dependency: name_str.clone(), error: e.to_string() })?;
        
        Ok(Self {
            name: name_str,
            version_req,
            optional: false,
            features: Vec::new(),
        })
    }

    /// Creates a new optional dependency.
    pub fn optional(name: impl Into<String>, version_req: impl Into<String>) -> Result<Self, DependencyError> {
        let mut dep = Self::new(name, version_req)?;
        dep.optional = true;
        Ok(dep)
    }

    /// Adds features to this dependency.
    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }

    /// Checks if a given version satisfies this dependency's version requirement.
    pub fn matches(&self, version: &Version) -> bool {
        self.version_req.matches(version)
    }
}

/// A node in the dependency graph representing a skill and its dependencies.
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// The skill ID.
    pub skill_id: String,
    /// The skill version.
    pub version: Version,
    /// Direct dependencies of this skill.
    pub dependencies: Vec<Dependency>,
    /// Metadata about the skill.
    pub metadata: Option<SkillMetadata>,
}

impl DependencyNode {
    /// Creates a new dependency node.
    pub fn new(skill_id: impl Into<String>, version: Version, dependencies: Vec<Dependency>) -> Self {
        Self {
            skill_id: skill_id.into(),
            version,
            dependencies,
            metadata: None,
        }
    }

    /// Adds metadata to this node.
    pub fn with_metadata(mut self, metadata: SkillMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Metadata about a skill for dependency resolution purposes.
#[derive(Debug, Clone, Default)]
pub struct SkillMetadata {
    /// Display name of the skill.
    pub name: String,
    /// Description of the skill.
    pub description: String,
    /// Category of the skill.
    pub category: String,
    /// Author of the skill.
    pub author: Option<String>,
}

/// A graph representing skill dependencies.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// All nodes in the graph, keyed by skill ID.
    nodes: HashMap<String, DependencyNode>,
    /// Adjacency list: skill_id -> list of dependency names.
    edges: HashMap<String, Vec<String>>,
    /// Reverse edges for finding dependents: skill_id -> list of dependent skill IDs.
    reverse_edges: HashMap<String, Vec<String>>,
    /// Available versions for each skill: skill_id -> list of versions.
    available_versions: HashMap<String, Vec<Version>>,
}

impl DependencyGraph {
    /// Creates a new empty dependency graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a node to the graph.
    pub fn add_node(&mut self, node: DependencyNode) {
        let skill_id = node.skill_id.clone();
        
        // Build edges from dependencies
        let dep_names: Vec<String> = node.dependencies.iter().map(|d| d.name.clone()).collect();
        
        // Update reverse edges
        for dep_name in &dep_names {
            self.reverse_edges
                .entry(dep_name.clone())
                .or_insert_with(Vec::new)
                .push(skill_id.clone());
        }
        
        self.edges.insert(skill_id.clone(), dep_names);
        self.nodes.insert(skill_id, node);
    }

    /// Registers available versions for a skill.
    pub fn register_versions(&mut self, skill_id: impl Into<String>, versions: Vec<Version>) {
        self.available_versions.insert(skill_id.into(), versions);
    }

    /// Gets a node by skill ID.
    pub fn get_node(&self, skill_id: &str) -> Option<&DependencyNode> {
        self.nodes.get(skill_id)
    }

    /// Gets all nodes in the graph.
    pub fn nodes(&self) -> impl Iterator<Item = &DependencyNode> {
        self.nodes.values()
    }

    /// Gets the dependencies of a skill.
    pub fn dependencies(&self, skill_id: &str) -> Option<&Vec<String>> {
        self.edges.get(skill_id)
    }

    /// Gets the dependents of a skill (skills that depend on it).
    pub fn dependents(&self, skill_id: &str) -> Option<&Vec<String>> {
        self.reverse_edges.get(skill_id)
    }

    /// Returns the number of nodes in the graph.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if the graph is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Errors that can occur during dependency resolution.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DependencyError {
    /// A circular dependency was detected.
    #[error("Circular dependency detected: {cycle:?}")]
    CircularDependency { cycle: Vec<String> },

    /// A version conflict was detected.
    #[error("Version conflict for dependency '{dependency}': required {required}, but found {found}")]
    VersionConflict {
        dependency: String,
        required: String,
        found: String,
    },

    /// A required dependency was not found.
    #[error("Missing dependency: {dependency} (required by {required_by})")]
    MissingDependency {
        dependency: String,
        required_by: String,
    },

    /// An optional dependency was not found.
    #[error("Optional dependency not found: {dependency} (requested by {requested_by})")]
    OptionalDependencyNotFound {
        dependency: String,
        requested_by: String,
    },

    /// Invalid version specification.
    #[error("Invalid version for dependency '{dependency}': {error}")]
    InvalidVersion { dependency: String, error: String },

    /// No matching version found for a dependency.
    #[error("No matching version found for dependency '{dependency}' (requirement: {requirement})")]
    NoMatchingVersion { dependency: String, requirement: String },

    /// Dependency resolution failed.
    #[error("Dependency resolution failed: {message}")]
    ResolutionFailed { message: String },

    /// Self-dependency detected.
    #[error("Self-dependency detected: {skill_id} depends on itself")]
    SelfDependency { skill_id: String },
}

/// Result of dependency resolution.
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    /// The ordered list of skills to install/activate.
    pub ordered_skills: Vec<ResolvedSkill>,
    /// Any warnings generated during resolution (e.g., optional deps not found).
    pub warnings: Vec<String>,
    /// Conflicts that were detected but may have been resolved.
    pub conflicts: Vec<VersionConflict>,
}

impl ResolutionResult {
    /// Creates a new resolution result.
    pub fn new(ordered_skills: Vec<ResolvedSkill>) -> Self {
        Self {
            ordered_skills,
            warnings: Vec::new(),
            conflicts: Vec::new(),
        }
    }

    /// Adds a warning to the result.
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Adds a conflict to the result.
    pub fn add_conflict(&mut self, conflict: VersionConflict) {
        self.conflicts.push(conflict);
    }
}

/// A resolved skill with its specific version.
#[derive(Debug, Clone)]
pub struct ResolvedSkill {
    /// The skill ID.
    pub skill_id: String,
    /// The resolved version.
    pub version: Version,
    /// Whether this skill is optional and was not found.
    pub optional_skipped: bool,
    /// The dependencies that this skill requires.
    pub dependencies: Vec<Dependency>,
}

impl ResolvedSkill {
    /// Creates a new resolved skill.
    pub fn new(skill_id: impl Into<String>, version: Version, dependencies: Vec<Dependency>) -> Self {
        Self {
            skill_id: skill_id.into(),
            version,
            dependencies,
            optional_skipped: false,
        }
    }

    /// Marks this skill as optional and skipped.
    pub fn mark_optional_skipped(mut self) -> Self {
        self.optional_skipped = true;
        self
    }
}

/// Represents a version conflict between different requirements.
#[derive(Debug, Clone)]
pub struct VersionConflict {
    /// The dependency that has conflicting requirements.
    pub dependency: String,
    /// The required versions from different skills.
    pub requirements: Vec<(String, VersionReq)>,
    /// The resolved version, if any.
    pub resolved_version: Option<Version>,
}

impl VersionConflict {
    /// Creates a new version conflict.
    pub fn new(dependency: impl Into<String>, requirements: Vec<(String, VersionReq)>) -> Self {
        Self {
            dependency: dependency.into(),
            requirements,
            resolved_version: None,
        }
    }

    /// Sets the resolved version.
    pub fn with_resolved_version(mut self, version: Version) -> Self {
        self.resolved_version = Some(version);
        self
    }
}

/// The dependency resolver that handles resolution algorithms.
#[derive(Debug, Clone)]
pub struct DependencyResolver {
    /// The dependency graph to resolve.
    graph: DependencyGraph,
    /// Whether to include optional dependencies.
    include_optional: bool,
    /// Maximum depth for transitive resolution.
    max_depth: usize,
}

impl DependencyResolver {
    /// Creates a new resolver with the given dependency graph.
    pub fn new(graph: DependencyGraph) -> Self {
        Self {
            graph,
            include_optional: true,
            max_depth: 100, // Prevent infinite recursion
        }
    }

    /// Sets whether to include optional dependencies.
    pub fn with_optional_dependencies(mut self, include: bool) -> Self {
        self.include_optional = include;
        self
    }

    /// Sets the maximum depth for transitive resolution.
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Detects cycles in the dependency graph.
    /// Returns all cycles found.
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();
        let mut path = Vec::new();

        for skill_id in self.graph.nodes.keys() {
            if !visited.contains(skill_id) {
                self.detect_cycles_from(skill_id, &mut visited, &mut in_stack, &mut path, &mut cycles);
            }
        }

        cycles
    }

    fn detect_cycles_from(
        &self,
        current: &str,
        visited: &mut HashSet<String>,
        in_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(current.to_string());
        in_stack.insert(current.to_string());
        path.push(current.to_string());

        if let Some(dependencies) = self.graph.dependencies(current) {
            for dep in dependencies {
                if !visited.contains(dep) {
                    self.detect_cycles_from(dep, visited, in_stack, path, cycles);
                } else if in_stack.contains(dep) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|x| x == dep).unwrap();
                    let cycle = path[cycle_start..].to_vec();
                    cycles.push(cycle);
                }
            }
        }

        path.pop();
        in_stack.remove(current);
    }

    /// Performs a topological sort of the dependency graph.
    /// Returns an error if a cycle is detected.
    /// The result is ordered so that dependencies come before dependents.
    pub fn topological_sort(&self) -> Result<Vec<String>, DependencyError> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut result = Vec::new();

        // Initialize in-degrees: a skill that depends on others has in-degree
        // equal to the number of its dependencies
        for skill_id in self.graph.nodes.keys() {
            let deps_count = self.graph.dependencies(skill_id).map(|d| d.len()).unwrap_or(0);
            in_degree.insert(skill_id.clone(), deps_count);
        }

        // Kahn's algorithm: start with skills that have no dependencies (in_degree = 0)
        let mut queue = VecDeque::new();
        for (skill_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(skill_id.clone());
            }
        }

        while let Some(skill_id) = queue.pop_front() {
            result.push(skill_id.clone());

            // For each skill that depends on this one, decrease its in-degree
            if let Some(dependents) = self.graph.dependents(&skill_id) {
                for dependent in dependents {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }

        if result.len() != self.graph.len() {
            // There's a cycle
            let cycles = self.detect_cycles();
            if let Some(cycle) = cycles.first() {
                return Err(DependencyError::CircularDependency {
                    cycle: cycle.clone(),
                });
            }
            return Err(DependencyError::ResolutionFailed {
                message: "Topological sort failed - cycle detected".to_string(),
            });
        }

        Ok(result)
    }

    /// Finds all version conflicts in the dependency graph.
    pub fn find_conflicts(&self) -> Vec<VersionConflict> {
        let mut conflicts = Vec::new();
        let mut dep_requirements: HashMap<String, Vec<(String, VersionReq)>> = HashMap::new();

        // Collect all version requirements for each dependency
        for (skill_id, node) in &self.graph.nodes {
            for dep in &node.dependencies {
                dep_requirements
                    .entry(dep.name.clone())
                    .or_insert_with(Vec::new)
                    .push((skill_id.clone(), dep.version_req.clone()));
            }
        }

        // Check for conflicts
        for (dep_name, requirements) in dep_requirements {
            if requirements.len() <= 1 {
                continue;
            }

            // Check if there's a version that satisfies all requirements
            let available = self.graph.available_versions.get(&dep_name);
            let mut has_conflict = false;

            if let Some(versions) = available {
                let mut compatible = false;
                for version in versions {
                    let satisfies_all = requirements
                        .iter()
                        .all(|(_, req)| req.matches(version));
                    if satisfies_all {
                        compatible = true;
                        break;
                    }
                }
                if !compatible {
                    has_conflict = true;
                }
            } else {
                // No available versions info, check if requirements are compatible
                let req_strs: HashSet<String> = requirements
                    .iter()
                    .map(|(_, req)| req.to_string())
                    .collect();
                if req_strs.len() > 1 {
                    has_conflict = true;
                }
            }

            if has_conflict {
                conflicts.push(VersionConflict::new(dep_name, requirements));
            }
        }

        conflicts
    }

    /// Resolves all dependencies starting from the given skill IDs.
    pub fn resolve_dependencies(&self, skill_ids: &[String]) -> Result<ResolutionResult, DependencyError> {
        let mut result = ResolutionResult::new(Vec::new());
        let mut resolved = HashMap::new();
        let mut visiting = HashSet::new();

        // First, check for cycles
        let cycles = self.detect_cycles();
        if !cycles.is_empty() {
            return Err(DependencyError::CircularDependency {
                cycle: cycles[0].clone(),
            });
        }

        // Resolve each requested skill
        for skill_id in skill_ids {
            if !resolved.contains_key(skill_id) {
                self.resolve_skill(skill_id, &mut resolved, &mut visiting, &mut result, 0)?;
            }
        }

        // Perform topological sort on resolved skills
        let mut sorted_graph = DependencyGraph::new();
        let resolved_keys: HashSet<_> = resolved.keys().cloned().collect();
        for (skill_id, resolved_skill) in &resolved {
            // Only include dependencies that are also in the resolved set
            let node = self.graph.get_node(skill_id);
            let deps: Vec<Dependency> = node
                .map(|n| n.dependencies.clone())
                .unwrap_or_default()
                .into_iter()
                .filter(|d| resolved_keys.contains(&d.name))
                .collect();
            sorted_graph.add_node(DependencyNode::new(skill_id, resolved_skill.version.clone(), deps));
        }

        let sorted_resolver = DependencyResolver::new(sorted_graph);
        let sorted_order = sorted_resolver.topological_sort()?;

        // Build the final ordered list
        let ordered_skills: Vec<ResolvedSkill> = sorted_order
            .into_iter()
            .filter_map(|id| resolved.remove(&id))
            .collect();

        result.ordered_skills = ordered_skills;
        Ok(result)
    }

    fn resolve_skill(
        &self,
        skill_id: &str,
        resolved: &mut HashMap<String, ResolvedSkill>,
        visiting: &mut HashSet<String>,
        result: &mut ResolutionResult,
        depth: usize,
    ) -> Result<(), DependencyError> {
        if depth > self.max_depth {
            return Err(DependencyError::ResolutionFailed {
                message: format!("Maximum resolution depth ({}) exceeded", self.max_depth),
            });
        }

        // Check for self-dependency
        if let Some(node) = self.graph.get_node(skill_id) {
            for dep in &node.dependencies {
                if dep.name == skill_id {
                    return Err(DependencyError::SelfDependency {
                        skill_id: skill_id.to_string(),
                    });
                }
            }
        }

        // Already resolved
        if resolved.contains_key(skill_id) {
            return Ok(());
        }

        // Check for cycle during resolution
        if visiting.contains(skill_id) {
            return Err(DependencyError::CircularDependency {
                cycle: vec![skill_id.to_string()],
            });
        }

        visiting.insert(skill_id.to_string());

        // Get the node
        let node = self
            .graph
            .get_node(skill_id)
            .ok_or_else(|| DependencyError::MissingDependency {
                dependency: skill_id.to_string(),
                required_by: "root".to_string(),
            })?;

        // Resolve dependencies first
        for dep in &node.dependencies {
            // Skip optional dependencies if not included
            if dep.optional && !self.include_optional {
                result.add_warning(format!(
                    "Skipping optional dependency '{}' (not including optional dependencies)",
                    dep.name
                ));
                continue;
            }

            // Check if the dependency exists
            let dep_node = self.graph.get_node(&dep.name);
            if dep_node.is_none() {
                if dep.optional {
                    result.add_warning(format!(
                        "Optional dependency '{}' not found (requested by {})",
                        dep.name, skill_id
                    ));
                    continue;
                }
                return Err(DependencyError::MissingDependency {
                    dependency: dep.name.clone(),
                    required_by: skill_id.to_string(),
                });
            }

            // Find a compatible version
            let dep_node = dep_node.unwrap();
            let compatible_version = self.find_compatible_version(&dep.name, &dep.version_req)?;

            // Check if already resolved with a different version
            if let Some(existing) = resolved.get(&dep.name) {
                if existing.version != compatible_version {
                    // Version conflict
                    let conflict = VersionConflict::new(
                        &dep.name,
                        vec![
                            (skill_id.to_string(), dep.version_req.clone()),
                            ("existing".to_string(), VersionReq::parse(&format!("={}", existing.version)).unwrap()),
                        ],
                    );
                    result.add_conflict(conflict);
                }
            } else {
                // Recursively resolve transitive dependencies first
                self.resolve_skill(&dep.name, resolved, visiting, result, depth + 1)?;
                
                // Then add this dependency to resolved (after its own deps are resolved)
                let resolved_dep = ResolvedSkill::new(
                    &dep.name,
                    compatible_version,
                    dep_node.dependencies.clone(),
                );
                resolved.insert(dep.name.clone(), resolved_dep);
            }
        }

        visiting.remove(skill_id);

        // Add this skill to resolved
        let resolved_skill = ResolvedSkill::new(skill_id, node.version.clone(), node.dependencies.clone());
        resolved.insert(skill_id.to_string(), resolved_skill);

        Ok(())
    }

    fn find_compatible_version(
        &self,
        dependency: &str,
        requirement: &VersionReq,
    ) -> Result<Version, DependencyError> {
        if let Some(versions) = self.graph.available_versions.get(dependency) {
            // Find the highest version that satisfies the requirement
            let mut compatible: Vec<&Version> = versions
                .iter()
                .filter(|v| requirement.matches(v))
                .collect();
            
            compatible.sort_by(|a, b| b.cmp(a)); // Sort descending

            compatible
                .first()
                .map(|v| (*v).clone())
                .ok_or_else(|| DependencyError::NoMatchingVersion {
                    dependency: dependency.to_string(),
                    requirement: requirement.to_string(),
                })
        } else {
            // If no version info available, assume the version in the graph is compatible
            self.graph
                .get_node(dependency)
                .map(|n| n.version.clone())
                .ok_or_else(|| DependencyError::MissingDependency {
                    dependency: dependency.to_string(),
                    required_by: "version_resolver".to_string(),
                })
        }
    }

    /// Gets all transitive dependencies of a skill.
    pub fn get_transitive_dependencies(&self, skill_id: &str) -> Result<Vec<String>, DependencyError> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        if let Some(dependencies) = self.graph.dependencies(skill_id) {
            for dep in dependencies {
                if !visited.contains(dep) {
                    visited.insert(dep.clone());
                    queue.push_back(dep.clone());
                    result.push(dep.clone());
                }
            }
        }

        while let Some(current) = queue.pop_front() {
            if let Some(dependencies) = self.graph.dependencies(&current) {
                for dep in dependencies {
                    if !visited.contains(dep) {
                        visited.insert(dep.clone());
                        queue.push_back(dep.clone());
                        result.push(dep.clone());
                    }
                }
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_creation() {
        let dep = Dependency::new("logging", ">=1.0.0").unwrap();
        assert_eq!(dep.name, "logging");
        assert_eq!(dep.version_req.to_string(), ">=1.0.0");
        assert!(!dep.optional);
        assert!(dep.features.is_empty());
    }

    #[test]
    fn test_optional_dependency() {
        let dep = Dependency::optional("cache", "^2.0").unwrap();
        assert_eq!(dep.name, "cache");
        assert!(dep.optional);
    }

    #[test]
    fn test_dependency_with_features() {
        let dep = Dependency::new("database", ">=3.0.0")
            .unwrap()
            .with_features(vec!["postgres".to_string(), "async".to_string()]);
        assert_eq!(dep.features.len(), 2);
        assert!(dep.features.contains(&"postgres".to_string()));
    }

    #[test]
    fn test_dependency_version_matching() {
        let dep = Dependency::new("logging", ">=1.0.0").unwrap();
        
        let v1 = Version::parse("0.9.0").unwrap();
        assert!(!dep.matches(&v1));
        
        let v2 = Version::parse("1.0.0").unwrap();
        assert!(dep.matches(&v2));
        
        let v3 = Version::parse("2.5.0").unwrap();
        assert!(dep.matches(&v3));
    }

    #[test]
    fn test_dependency_invalid_version() {
        let result = Dependency::new("test", "invalid");
        assert!(result.is_err());
        assert!(matches!(result, Err(DependencyError::InvalidVersion { .. })));
    }

    #[test]
    fn test_graph_creation_and_nodes() {
        let mut graph = DependencyGraph::new();
        
        let node = DependencyNode::new(
            "skill-a",
            Version::parse("1.0.0").unwrap(),
            vec![
                Dependency::new("skill-b", ">=1.0.0").unwrap(),
                Dependency::new("skill-c", "^2.0").unwrap(),
            ],
        );
        
        graph.add_node(node);
        
        assert_eq!(graph.len(), 1);
        assert!(graph.get_node("skill-a").is_some());
        assert!(graph.get_node("skill-b").is_none());
    }

    #[test]
    fn test_graph_edges() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "parent",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("child", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "child",
            Version::parse("1.0.0").unwrap(),
            vec![],
        ));
        
        assert_eq!(graph.dependencies("parent").unwrap().len(), 1);
        assert_eq!(graph.dependents("child").unwrap().len(), 1);
    }

    #[test]
    fn test_graph_version_registration() {
        let mut graph = DependencyGraph::new();
        
        graph.register_versions(
            "logging",
            vec![
                Version::parse("1.0.0").unwrap(),
                Version::parse("1.5.0").unwrap(),
                Version::parse("2.0.0").unwrap(),
            ],
        );
        
        let versions = graph.available_versions.get("logging").unwrap();
        assert_eq!(versions.len(), 3);
    }

    #[test]
    fn test_no_cycles() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "a",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("b", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "b",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("c", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "c",
            Version::parse("1.0.0").unwrap(),
            vec![],
        ));
        
        let resolver = DependencyResolver::new(graph);
        let cycles = resolver.detect_cycles();
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_direct_cycle() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "a",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("b", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "b",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("a", ">=1.0.0").unwrap()],
        ));
        
        let resolver = DependencyResolver::new(graph);
        let cycles = resolver.detect_cycles();
        assert!(!cycles.is_empty());
        assert_eq!(cycles.len(), 1);
        assert!(cycles[0].contains(&"a".to_string()));
        assert!(cycles[0].contains(&"b".to_string()));
    }

    #[test]
    fn test_indirect_cycle() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "a",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("b", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "b",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("c", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "c",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("a", ">=1.0.0").unwrap()],
        ));
        
        let resolver = DependencyResolver::new(graph);
        let cycles = resolver.detect_cycles();
        assert!(!cycles.is_empty());
        assert_eq!(cycles.len(), 1);
    }

    #[test]
    fn test_self_dependency() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "self-dep",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("self-dep", ">=1.0.0").unwrap()],
        ));
        
        let resolver = DependencyResolver::new(graph);
        let cycles = resolver.detect_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_topological_sort_simple() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "app",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("framework", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "framework",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("utils", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "utils",
            Version::parse("1.0.0").unwrap(),
            vec![],
        ));
        
        let resolver = DependencyResolver::new(graph);
        let sorted = resolver.topological_sort().unwrap();
        
        let utils_idx = sorted.iter().position(|x| x == "utils").unwrap();
        let framework_idx = sorted.iter().position(|x| x == "framework").unwrap();
        let app_idx = sorted.iter().position(|x| x == "app").unwrap();
        
        assert!(utils_idx < framework_idx);
        assert!(framework_idx < app_idx);
    }

    #[test]
    fn test_topological_sort_with_cycle() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "a",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("b", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "b",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("a", ">=1.0.0").unwrap()],
        ));
        
        let resolver = DependencyResolver::new(graph);
        let result = resolver.topological_sort();
        assert!(result.is_err());
        assert!(matches!(result, Err(DependencyError::CircularDependency { .. })));
    }

    #[test]
    fn test_no_conflicts() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "app",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("logging", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "plugin",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("logging", ">=1.5.0").unwrap()],
        ));
        
        graph.register_versions(
            "logging",
            vec![
                Version::parse("1.0.0").unwrap(),
                Version::parse("1.5.0").unwrap(),
                Version::parse("2.0.0").unwrap(),
            ],
        );
        
        let resolver = DependencyResolver::new(graph);
        let conflicts = resolver.find_conflicts();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_version_conflict() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "app",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("logging", ">=1.0.0,<2.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "plugin",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("logging", ">=2.0.0").unwrap()],
        ));
        
        graph.register_versions(
            "logging",
            vec![
                Version::parse("1.5.0").unwrap(),
                Version::parse("2.5.0").unwrap(),
            ],
        );
        
        let resolver = DependencyResolver::new(graph);
        let conflicts = resolver.find_conflicts();
        assert!(!conflicts.is_empty());
        assert_eq!(conflicts[0].dependency, "logging");
    }

    #[test]
    fn test_resolve_simple_chain() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "app",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("framework", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "framework",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("utils", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "utils",
            Version::parse("1.0.0").unwrap(),
            vec![],
        ));
        
        graph.register_versions("framework", vec![Version::parse("1.0.0").unwrap()]);
        graph.register_versions("utils", vec![Version::parse("1.0.0").unwrap()]);
        
        let resolver = DependencyResolver::new(graph);
        let result = resolver.resolve_dependencies(&["app".to_string()]).unwrap();
        
        assert_eq!(result.ordered_skills.len(), 3);
        assert_eq!(result.ordered_skills[0].skill_id, "utils");
        assert_eq!(result.ordered_skills[1].skill_id, "framework");
        assert_eq!(result.ordered_skills[2].skill_id, "app");
    }

    #[test]
    fn test_resolve_with_optional_dependencies() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "app",
            Version::parse("1.0.0").unwrap(),
            vec![
                Dependency::new("required", ">=1.0.0").unwrap(),
                Dependency::optional("optional-dep", ">=1.0.0").unwrap(),
            ],
        ));
        
        graph.add_node(DependencyNode::new(
            "required",
            Version::parse("1.0.0").unwrap(),
            vec![],
        ));
        
        let resolver = DependencyResolver::new(graph);
        let result = resolver.resolve_dependencies(&["app".to_string()]).unwrap();
        
        assert_eq!(result.ordered_skills.len(), 2);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("optional-dep"));
    }

    #[test]
    fn test_resolve_without_optional_dependencies() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "app",
            Version::parse("1.0.0").unwrap(),
            vec![
                Dependency::new("required", ">=1.0.0").unwrap(),
                Dependency::optional("optional-dep", ">=1.0.0").unwrap(),
            ],
        ));
        
        graph.add_node(DependencyNode::new(
            "required",
            Version::parse("1.0.0").unwrap(),
            vec![],
        ));
        
        let resolver = DependencyResolver::new(graph).with_optional_dependencies(false);
        let result = resolver.resolve_dependencies(&["app".to_string()]).unwrap();
        
        assert_eq!(result.ordered_skills.len(), 2);
        assert!(result.warnings.iter().any(|w| w.contains("optional-dep")));
    }

    #[test]
    fn test_resolve_missing_required_dependency() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "app",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("missing", ">=1.0.0").unwrap()],
        ));
        
        let resolver = DependencyResolver::new(graph);
        let result = resolver.resolve_dependencies(&["app".to_string()]);
        
        assert!(result.is_err());
        assert!(matches!(result, Err(DependencyError::MissingDependency { .. })));
    }

    #[test]
    fn test_resolve_with_version_selection() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "app",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("logging", ">=1.5.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "logging",
            Version::parse("1.5.0").unwrap(),
            vec![],
        ));
        
        graph.register_versions(
            "logging",
            vec![
                Version::parse("1.0.0").unwrap(),
                Version::parse("1.5.0").unwrap(),
                Version::parse("2.0.0").unwrap(),
            ],
        );
        
        let resolver = DependencyResolver::new(graph);
        let result = resolver.resolve_dependencies(&["app".to_string()]).unwrap();
        
        let logging = result.ordered_skills.iter().find(|s| s.skill_id == "logging").unwrap();
        assert_eq!(logging.version, Version::parse("2.0.0").unwrap());
    }

    #[test]
    fn test_resolve_circular_dependency_error() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "a",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("b", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "b",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("a", ">=1.0.0").unwrap()],
        ));
        
        let resolver = DependencyResolver::new(graph);
        let result = resolver.resolve_dependencies(&["a".to_string()]);
        
        assert!(result.is_err());
        assert!(matches!(result, Err(DependencyError::CircularDependency { .. })));
    }

    #[test]
    fn test_transitive_dependencies() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "app",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("framework", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "framework",
            Version::parse("1.0.0").unwrap(),
            vec![
                Dependency::new("utils", ">=1.0.0").unwrap(),
                Dependency::new("logging", ">=1.0.0").unwrap(),
            ],
        ));
        
        graph.add_node(DependencyNode::new("utils", Version::parse("1.0.0").unwrap(), vec![]));
        graph.add_node(DependencyNode::new("logging", Version::parse("1.0.0").unwrap(), vec![]));
        
        let resolver = DependencyResolver::new(graph);
        let transitive = resolver.get_transitive_dependencies("app").unwrap();
        
        assert!(transitive.contains(&"framework".to_string()));
        assert!(transitive.contains(&"utils".to_string()));
        assert!(transitive.contains(&"logging".to_string()));
        assert_eq!(transitive.len(), 3);
    }

    #[test]
    fn test_transitive_dependencies_diamond() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "app",
            Version::parse("1.0.0").unwrap(),
            vec![
                Dependency::new("left", ">=1.0.0").unwrap(),
                Dependency::new("right", ">=1.0.0").unwrap(),
            ],
        ));
        
        graph.add_node(DependencyNode::new(
            "left",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("bottom", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "right",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("bottom", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new("bottom", Version::parse("1.0.0").unwrap(), vec![]));
        
        let resolver = DependencyResolver::new(graph);
        let transitive = resolver.get_transitive_dependencies("app").unwrap();
        
        assert!(transitive.contains(&"left".to_string()));
        assert!(transitive.contains(&"right".to_string()));
        assert!(transitive.contains(&"bottom".to_string()));
        assert_eq!(transitive.iter().filter(|s| *s == "bottom").count(), 1);
    }

    #[test]
    fn test_complex_dependency_tree() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "app",
            Version::parse("1.0.0").unwrap(),
            vec![
                Dependency::new("framework", ">=1.0.0").unwrap(),
                Dependency::new("plugin", ">=1.0.0").unwrap(),
            ],
        ));
        
        graph.add_node(DependencyNode::new(
            "framework",
            Version::parse("1.0.0").unwrap(),
            vec![
                Dependency::new("utils", ">=1.0.0").unwrap(),
                Dependency::new("logging", ">=1.0.0").unwrap(),
            ],
        ));
        
        graph.add_node(DependencyNode::new(
            "plugin",
            Version::parse("1.0.0").unwrap(),
            vec![
                Dependency::new("logging", ">=1.0.0").unwrap(),
                Dependency::new("cache", ">=1.0.0").unwrap(),
            ],
        ));
        
        graph.add_node(DependencyNode::new(
            "utils",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("http", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new("logging", Version::parse("1.0.0").unwrap(), vec![]));
        graph.add_node(DependencyNode::new("cache", Version::parse("1.0.0").unwrap(), vec![]));
        graph.add_node(DependencyNode::new("http", Version::parse("1.0.0").unwrap(), vec![]));
        
        let resolver = DependencyResolver::new(graph);
        let result = resolver.resolve_dependencies(&["app".to_string()]).unwrap();
        
        assert_eq!(result.ordered_skills.len(), 7);
        
        let get_idx = |id: &str| -> usize {
            result.ordered_skills.iter().position(|s| s.skill_id == id).unwrap()
        };
        
        assert!(get_idx("http") < get_idx("utils"));
        assert!(get_idx("utils") < get_idx("framework"));
        assert!(get_idx("logging") < get_idx("framework"));
        assert!(get_idx("cache") < get_idx("plugin"));
        assert!(get_idx("framework") < get_idx("app"));
        assert!(get_idx("plugin") < get_idx("app"));
    }

    #[test]
    fn test_multiple_roots_resolution() {
        let mut graph = DependencyGraph::new();
        
        graph.add_node(DependencyNode::new(
            "app1",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("shared", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new(
            "app2",
            Version::parse("1.0.0").unwrap(),
            vec![Dependency::new("shared", ">=1.0.0").unwrap()],
        ));
        
        graph.add_node(DependencyNode::new("shared", Version::parse("1.0.0").unwrap(), vec![]));
        
        let resolver = DependencyResolver::new(graph);
        let result = resolver.resolve_dependencies(&["app1".to_string(), "app2".to_string()]).unwrap();
        
        let shared_count = result.ordered_skills.iter().filter(|s| s.skill_id == "shared").count();
        assert_eq!(shared_count, 1);
        assert_eq!(result.ordered_skills.len(), 3);
    }

    #[test]
    fn test_dependency_equality() {
        let dep1 = Dependency::new("logging", ">=1.0.0").unwrap();
        let dep2 = Dependency::new("logging", ">=1.0.0").unwrap();
        let dep3 = Dependency::new("logging", ">=2.0.0").unwrap();
        
        assert_eq!(dep1, dep2);
        assert_ne!(dep1, dep3);
    }

    #[test]
    fn test_resolution_result_warnings() {
        let mut result = ResolutionResult::new(Vec::new());
        result.add_warning("Test warning");
        result.add_conflict(VersionConflict::new("test", vec![]));
        
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.conflicts.len(), 1);
    }

    #[test]
    fn test_resolved_skill_optional_skipped() {
        let skill = ResolvedSkill::new("test", Version::parse("1.0.0").unwrap(), vec![])
            .mark_optional_skipped();
        
        assert!(skill.optional_skipped);
    }

    #[test]
    fn test_version_conflict_with_resolved_version() {
        let conflict = VersionConflict::new("logging", vec![])
            .with_resolved_version(Version::parse("1.5.0").unwrap());
        
        assert_eq!(conflict.resolved_version, Some(Version::parse("1.5.0").unwrap()));
    }

    #[test]
    fn test_empty_graph() {
        let graph = DependencyGraph::new();
        let resolver = DependencyResolver::new(graph);
        
        assert!(resolver.topological_sort().unwrap().is_empty());
        assert!(resolver.detect_cycles().is_empty());
        assert!(resolver.find_conflicts().is_empty());
    }

    #[test]
    fn test_dependency_features_preserved() {
        let dep = Dependency::new("database", ">=1.0.0")
            .unwrap()
            .with_features(vec!["postgres".to_string(), "async".to_string()]);
        
        let mut graph = DependencyGraph::new();
        graph.add_node(DependencyNode::new(
            "app",
            Version::parse("1.0.0").unwrap(),
            vec![dep.clone()],
        ));
        graph.add_node(DependencyNode::new(
            "database",
            Version::parse("1.0.0").unwrap(),
            vec![],
        ));
        
        let resolver = DependencyResolver::new(graph);
        let result = resolver.resolve_dependencies(&["app".to_string()]).unwrap();
        
        let app = result.ordered_skills.iter().find(|s| s.skill_id == "app").unwrap();
        assert_eq!(app.dependencies[0].features.len(), 2);
    }
}
