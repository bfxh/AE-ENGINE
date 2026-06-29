#!/usr/bin/env python3
"""
GDScript Syntax Validator for Wasteland Project
Validates GDScript files without needing Godot engine
"""

import re
import sys
from pathlib import Path

def validate_gdscript(file_path):
    """Validate GDScript syntax"""
    errors = []
    warnings = []
    
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    # Skip validation for this scanner file itself
    if 'validate_gdscript' in str(file_path):
        return errors, warnings
    
    for i, line in enumerate(lines, 1):
        stripped = line.strip()
        
        # Check for _get_tree() which doesn't exist in Godot
        if '_get_tree()' in stripped:
            errors.append(f"Line {i}: _get_tree() method doesn't exist, use get_tree()")
        
        # Check for string multiplication (not supported in GDScript)
        # Pattern: literal string followed by * number (e.g., "=" * 60)
        if re.search(r'["\'][^*]+\*\s*\d+\s*[+\-]?\s*$', stripped):
            errors.append(f"Line {i}: String multiplication not supported in GDScript")
        
        # Check function definitions for type annotation issues
        if stripped.startswith('func '):
            match = re.match(r'func\s+(\w+)\s*\((.*)\)', stripped)
            if match:
                params = match.group(2)
                if ':' in params:
                    type_annotations = re.findall(r'(\w+)\s*:\s*(\w+)', params)
                    for param_name, param_type in type_annotations:
                        known_types = ['int', 'float', 'String', 'bool', 'Node', 'Node3D', 'Vector3', 'Vector2', 
                                      'Array', 'Dictionary', 'Object', 'Variant', 'Node2D', 'Control']
                        if param_type not in known_types and not param_type.endswith('='):
                            warnings.append(f"Line {i}: Unknown type annotation '{param_type}' for '{param_name}'")
    
    return errors, warnings

def main():
    project_path = Path(r'd:\rj\wasteland_project\godot_project\scripts')
    
    if not project_path.exists():
        print(f"Error: Project path not found: {project_path}")
        return 1
    
    all_errors = []
    all_warnings = []
    
    for gdscript in project_path.glob('*.gd'):
        errors, warnings = validate_gdscript(gdscript)
        if errors:
            all_errors.append((gdscript.name, errors))
        if warnings:
            all_warnings.append((gdscript.name, warnings))
    
    print("=" * 60)
    print("GDSCRIPT SYNTAX VALIDATION REPORT")
    print("=" * 60)
    
    if all_errors:
        print(f"\nERRORS ({sum(len(e) for _, e in all_errors)}):")
        for filename, errors in all_errors:
            print(f"\n  {filename}:")
            for error in errors:
                print(f"    [ERROR] {error}")
    
    if all_warnings:
        print(f"\nWARNINGS ({sum(len(w) for _, w in all_warnings)}):")
        for filename, warnings in all_warnings:
            print(f"\n  {filename}:")
            for warning in warnings:
                print(f"    [WARN] {warning}")
    
    if not all_errors and not all_warnings:
        print("\nNo syntax errors or warnings found!")
    
    print("\n" + "=" * 60)
    
    return 1 if all_errors else 0

if __name__ == '__main__':
    sys.exit(main())
