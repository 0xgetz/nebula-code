export interface SkillCard {
  id: string;
  name: string;
  description: string;
  version: string;
  author: SkillAuthor;
  category: SkillCategory;
  tags: string[];
  configSchema?: Record<string, unknown>;
  defaultConfig: Record<string, unknown>;
  dependencies: SkillDependency[];
  license: string;
  repository?: string;
  documentation?: string;
}

export interface SkillAuthor {
  name: string;
  email?: string;
  url?: string;
}

export type SkillCategory = 
  | 'code-generation'
  | 'code-review'
  | 'testing'
  | 'documentation'
  | 'deployment'
  | 'performance'
  | 'security'
  | 'data-processing'
  | 'utilities'
  | 'other';

export interface SkillDependency {
  skillId: string;
  version: string;
}

export interface Project {
  name: string;
  description: string;
  version: string;
  root: string;
  projectType: ProjectType;
  skills: InstalledSkill[];
  config: ProjectConfig;
}

export type ProjectType = 'web' | 'desktop' | 'cli' | 'library' | 'other';

export interface InstalledSkill {
  skillId: string;
  version: string;
  config: Record<string, unknown>;
  enabled: boolean;
}

export interface ProjectConfig {
  build: BuildConfig;
  test: TestConfig;
  deploy?: DeployConfig;
}

export interface BuildConfig {
  outputDir: string;
  entryPoint?: string;
  optimize: boolean;
}

export interface TestConfig {
  testDirs: string[];
  coverageThreshold?: number;
}

export interface DeployConfig {
  target: string;
  preDeploy: string[];
  postDeploy: string[];
}
