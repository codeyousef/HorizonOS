```
name: Build and Release HorizonOS ISO

on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to build (e.g., 0.1.0)'
        required: true
        default: '0.1.0'

jobs:
  build-iso:
    runs-on: ubuntu-latest
    container:
      image: archlinux:latest
      options: --privileged
    
    steps:
    - name: Install build dependencies
      run: |
        pacman -Syu --noconfirm
        pacman -S --noconfirm base-devel git archiso ostree btrfs-progs \
          dosfstools grub efibootmgr squashfs-tools sudo
    
    - name: Checkout code
      uses: actions/checkout@v4
    
    - name: Set version
      run: |
        if [ "${{ github.event_name }}" = "push" ]; then
          VERSION=${GITHUB_REF#refs/tags/v}
        else
          VERSION="${{ github.event.inputs.version }}"
        fi
        echo "VERSION=$VERSION" >> $GITHUB_ENV
        echo "Building HorizonOS version: $VERSION"
    
    - name: Initialize OSTree repository
      run: |
        ostree init --repo=repo --mode=archive
    
    - name: Build base image
      run: |
        chmod +x scripts/scripts/build-base-image.sh
        sudo ./scripts/scripts/build-base-image.sh
    
    - name: Build test system
      run: |
        chmod +x scripts/scripts/build-test.sh
        sudo ./scripts/scripts/build-test.sh
    
    - name: Build ISO
      run: |
        chmod +x scripts/scripts/build-iso.sh
        sudo ./scripts/scripts/build-iso.sh
    
    - name: Calculate checksums
      run: |
        cd build/out
        sha256sum horizonos-*.iso > horizonos-${{ env.VERSION }}.iso.sha256
        
    - name: Create OSTree bundle for updates
      run: |
        # Export the latest commit as a bundle for distribution
        COMMIT=$(ostree log --repo=repo horizonos/test/x86_64 | head -1 | cut -d' ' -f2)
        ostree --repo=repo static-delta generate horizonos/test/x86_64
        tar czf horizonos-ostree-${{ env.VERSION }}.tar.gz -C repo .
        mv horizonos-ostree-${{ env.VERSION }}.tar.gz build/out/
    
    - name: Upload artifacts
      uses: actions/upload-artifact@v3
      with:
        name: horizonos-${{ env.VERSION }}
        path: |
          build/out/horizonos-*.iso
          build/out/horizonos-*.iso.sha256
          build/out/horizonos-ostree-*.tar.gz

  create-release:
    needs: build-iso
    runs-on: ubuntu-latest
    permissions:
      contents: write
    
    steps:
    - name: Download artifacts
      uses: actions/download-artifact@v3
      with:
        name: horizonos-${{ env.VERSION }}
        path: ./release
    
    - name: Create Release
      uses: softprops/action-gh-release@v1
      with:
        tag_name: ${{ github.ref_name }}
        name: HorizonOS ${{ env.VERSION }}
        draft: false
        prerelease: true
        files: |
          release/horizonos-*.iso
          release/horizonos-*.iso.sha256
          release/horizonos-ostree-*.tar.gz
        body: |
          ## HorizonOS ${{ env.VERSION }}
          
          ### Installation
          1. Download the ISO file
          2. Write to USB: `sudo dd if=horizonos-*.iso of=/dev/sdX bs=4M status=progress`
          3. Boot from USB and run `horizonos-install`
          
          ### What's New
          - OSTree-based atomic updates
          - Container-based architecture
          - Kotlin DSL configuration system
          
          ### Downloads
          - **ISO**: For fresh installations
          - **OSTree Bundle**: For manual updates of existing systems
          - **SHA256**: Verify download integrity
```