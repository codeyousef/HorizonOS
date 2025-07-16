#!/bin/bash

echo "=== HorizonOS Development Environment Check ==="
echo ""

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

check_command() {
    if command -v $1 &> /dev/null; then
        echo -e "${GREEN}✓${NC} $1 is installed"
        return 0
    else
        echo -e "${RED}✗${NC} $1 is NOT installed"
        return 1
    fi
}

check_file() {
    if [ -f "$1" ]; then
        echo -e "${GREEN}✓${NC} $1 exists"
        return 0
    else
        echo -e "${RED}✗${NC} $1 is missing"
        return 1
    fi
}

check_dir() {
    if [ -d "$1" ]; then
        echo -e "${GREEN}✓${NC} $1 exists"
        return 0
    else
        echo -e "${RED}✗${NC} $1 is missing"
        return 1
    fi
}

errors=0

echo "1. Checking core tools..."
check_command git || ((errors++))
check_command ostree || ((errors++))
check_command pacstrap || ((errors++))
check_command mkarchiso || ((errors++))
check_command mksquashfs || ((errors++))

echo ""
echo "2. Checking development tools..."
check_command gcc || ((errors++))
check_command make || ((errors++))
check_command yay || ((errors++))
check_command code || check_command codium || echo "  (VS Code optional)"

echo ""
echo "3. Checking project structure..."
check_dir "scripts" || ((errors++))
check_dir "src" || ((errors++))
check_dir "docs" || ((errors++))
check_file "README.md" || ((errors++))

echo ""
echo "4. Checking build scripts..."
for script in scripts/*.sh; do
    if [ -f "$script" ]; then
        if [ -x "$script" ]; then
            echo -e "${GREEN}✓${NC} $script is executable"
        else
            echo -e "${RED}✗${NC} $script is not executable"
            ((errors++))
        fi
    fi
done

echo ""
echo "5. Checking OSTree repository..."
if [ -d "repo" ]; then
    if ostree --repo=repo refs &> /dev/null; then
        echo -e "${GREEN}✓${NC} OSTree repository is initialized"
        ostree --repo=repo refs 2>/dev/null | sed 's/^/  - /'
    else
        echo -e "${RED}✗${NC} OSTree repository not properly initialized"
        ((errors++))
    fi
else
    echo -e "${RED}✗${NC} OSTree repository directory missing"
    ((errors++))
fi

echo ""
echo "6. Checking system requirements..."
echo "  RAM: $(free -h | awk '/^Mem:/ {print $2}')"
echo "  CPU: $(nproc) cores"
echo "  Disk space in home: $(df -h ~ | awk 'NR==2 {print $4}' | sed 's/G/ GB/')"

echo ""
echo "7. Git status..."
if git rev-parse --git-dir > /dev/null 2>&1; then
    echo "  Branch: $(git branch --show-current)"
    echo "  Remote: $(git remote -v | grep origin | head -1 | awk '{print $2}')"
    if [ -n "$(git status --porcelain)" ]; then
        echo "  Working directory has uncommitted changes"
    else
        echo "  Working directory is clean"
    fi
fi

echo ""
echo "================================"
if [ $errors -eq 0 ]; then
    echo -e "${GREEN}All checks passed!${NC} Your development environment is ready."
    echo ""
    echo "You can now run:"
    echo "  ./scripts/build-test.sh    - Build a test OSTree commit"
    echo "  ./scripts/build-iso.sh     - Create a bootable ISO"
else
    echo -e "${RED}$errors issues found.${NC} Please install missing components."
fi