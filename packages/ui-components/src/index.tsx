import React from 'react';
import { SkillCard, SkillCategory } from '@nebula-code/shared-types';

export interface SkillCardProps {
  skill: SkillCard;
  onSelect?: (skill: SkillCard) => void;
  compact?: boolean;
}

export const SkillCard: React.FC<SkillCardProps> = ({ skill, onSelect, compact = false }) => {
  const handleClick = () => {
    if (onSelect) {
      onSelect(skill);
    }
  };

  const categoryColors: Record<SkillCategory, string> = {
    'code-generation': 'bg-blue-100 text-blue-800',
    'code-review': 'bg-green-100 text-green-800',
    'testing': 'bg-yellow-100 text-yellow-800',
    'documentation': 'bg-purple-100 text-purple-800',
    'deployment': 'bg-red-100 text-red-800',
    'performance': 'bg-orange-100 text-orange-800',
    'security': 'bg-gray-100 text-gray-800',
    'data-processing': 'bg-indigo-100 text-indigo-800',
    'utilities': 'bg-pink-100 text-pink-800',
    'other': 'bg-slate-100 text-slate-800',
  };

  if (compact) {
    return (
      <div 
        className="p-4 border rounded-lg hover:shadow-md transition-shadow cursor-pointer"
        onClick={handleClick}
      >
        <div className="flex items-center justify-between">
          <h3 className="font-semibold text-lg">{skill.name}</h3>
          <span className={`px-2 py-1 rounded text-xs ${categoryColors[skill.category] || categoryColors.other}`}>
            {skill.category}
          </span>
        </div>
        <p className="text-sm text-gray-600 mt-2">{skill.description}</p>
      </div>
    );
  }

  return (
    <div 
      className="p-6 border rounded-lg hover:shadow-lg transition-shadow cursor-pointer"
      onClick={handleClick}
    >
      <div className="flex items-start justify-between mb-4">
        <div>
          <h3 className="font-bold text-xl">{skill.name}</h3>
          <p className="text-sm text-gray-500">v{skill.version} by {skill.author.name}</p>
        </div>
        <span className={`px-3 py-1 rounded-full text-sm ${categoryColors[skill.category] || categoryColors.other}`}>
          {skill.category}
        </span>
      </div>
      
      <p className="text-gray-700 mb-4">{skill.description}</p>
      
      {skill.tags.length > 0 && (
        <div className="flex flex-wrap gap-2 mb-4">
          {skill.tags.map(tag => (
            <span key={tag} className="px-2 py-1 bg-gray-100 text-gray-700 text-xs rounded">
              {tag}
            </span>
          ))}
        </div>
      )}
      
      <div className="flex items-center justify-between text-sm text-gray-500">
        <span>License: {skill.license}</span>
        {skill.dependencies.length > 0 && (
          <span>{skill.dependencies.length} dependencies</span>
        )}
      </div>
    </div>
  );
};

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'outline';
  size?: 'sm' | 'md' | 'lg';
}

export const Button: React.FC<ButtonProps> = ({
  children,
  variant = 'primary',
  size = 'md',
  className = '',
  ...props
}) => {
  const baseStyles = 'inline-flex items-center justify-center font-medium rounded transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2';
  
  const variants = {
    primary: 'bg-blue-600 text-white hover:bg-blue-700 focus:ring-blue-500',
    secondary: 'bg-gray-200 text-gray-900 hover:bg-gray-300 focus:ring-gray-500',
    outline: 'border-2 border-blue-600 text-blue-600 hover:bg-blue-50 focus:ring-blue-500',
  };
  
  const sizes = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-base',
    lg: 'px-6 py-3 text-lg',
  };
  
  return (
    <button
      className={`${baseStyles} ${variants[variant]} ${sizes[size]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};

export { Card } from './Card';
export { Badge } from './Badge';
