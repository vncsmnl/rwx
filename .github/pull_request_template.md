## 📝 Description

<!-- Provide a brief description of the changes introduced by this PR. -->

## 🔗 Related Issues

<!-- Link any related issues below using standard GitHub keywords (e.g., Fixes #123, Closes #456, Ref #789). -->
Fixes #

## 🏷️ Type of Change

<!-- Please check the options that apply to this PR using [x]. -->

- [ ] 🐛 **Bug Fix**: Non-breaking change that fixes an issue
- [ ] 💡 **New Feature**: Non-breaking change that adds functionality
- [ ] ⚡ **Performance / Refactoring**: Code restructuring or optimization without behavior changes
- [ ] 🎨 **TUI / UX Improvement**: Visual tweaks, keybindings, layout updates
- [ ] 📚 **Documentation**: Updates to README, CLI help, or code docs
- [ ] ⚙️ **CI/CD / Dependencies**: Workflow updates, Cargo crate upgrades, build configuration
- [ ] 💥 **Breaking Change**: Fix or feature that would cause existing functionality to change

## 🧩 Affected Components

- [ ] Interactive TUI (File Browser, Layout, Keyboard Navigation)
- [ ] Permissions Grid Editor (`r/w/x` toggles)
- [ ] Octal Permissions Input / Parsing
- [ ] Special Bits (SetUID, SetGID, Sticky bit)
- [ ] Ownership Management (`chown` / `chgrp` / User & Group selection)
- [ ] Recursive Operations (`-R`)
- [ ] CLI Arguments & Terminal State Management
- [ ] Other / Infrastructure

## 🛠️ Changes Summary

<!-- Bullet points describing the key technical changes made in this PR. -->
- 

## 🧪 Testing & Verification

<!-- Describe how you tested your changes. Include unit tests, manual checks, or OS environments tested. -->

- [ ] Added / updated unit or integration tests
- [ ] Tested manually in terminal

**Tested OS / Environment:**
- [ ] Linux (x86_64 / ARM64)
- [ ] macOS (Apple Silicon / Intel)
- [ ] Other: 

**Verification Commands Executed:**
```bash
cargo test
cargo clippy --all-targets --all-features
cargo fmt --check
```

## 📸 Screenshots / GIFs (if applicable)

<!-- Add screenshots or screen recordings showing UI/UX changes if relevant. -->

## 📋 Pre-submission Checklist

- [ ] My code follows the code style guidelines of this project (`cargo fmt`).
- [ ] I have performed a self-review of my own code.
- [ ] I have commented my code, particularly in hard-to-understand areas.
- [ ] I have made corresponding changes to the documentation (if needed).
- [ ] My changes generate no new warnings (`cargo clippy`).
- [ ] I have added tests that prove my fix is effective or that my feature works.
- [ ] All new and existing tests passed (`cargo test`).
