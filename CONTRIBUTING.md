# Contributing to The Gem

Thank you for your interest in contributing to The Gem, BelizeChain's smart contract platform! 🎉

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/gem.git
   cd gem
   ```
3. **Install dependencies**:
   ```bash
   cargo install cargo-contract --force
   ```

## Development Workflow

### Building Contracts

```bash
cd dalla_token
cargo contract build --release
```

### Running Tests

```bash
cargo test
```

### Code Style

- Follow Rust standard formatting: `cargo fmt`
- Run clippy for linting: `cargo clippy`
- Write tests for all new features
- Add documentation comments for public APIs

## Submitting Changes

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**:
   - Write clear, concise commit messages
   - Include tests for new functionality
   - Update documentation as needed

3. **Test thoroughly**:
   ```bash
   cargo test --all
   cargo clippy --all-targets
   cargo fmt --check
   ```

4. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

5. **Create a Pull Request**:
   - Provide a clear description of the changes
   - Reference any related issues
   - Ensure all CI checks pass

## Contract Standards

### PSP22 (Fungible Tokens)
- Must implement all required PSP22 functions
- Include proper error handling
- Emit all required events
- Include comprehensive tests

### PSP34 (NFTs)
- Follow PSP34 standard completely
- Support metadata URIs
- Include enumeration functions
- Test edge cases thoroughly

### Best Practices

1. **Use saturating arithmetic**: Prevents overflow panics
   ```rust
   balance.saturating_add(amount)
   ```

2. **Emit events**: For all state changes
   ```rust
   self.env().emit_event(Transfer { ... });
   ```

3. **Check inputs first**: Before modifying state
   ```rust
   if amount == 0 {
       return Err(Error::ZeroAmount);
   }
   ```

4. **Write comprehensive tests**:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[ink::test]
       fn test_feature() {
           // Test implementation
       }
   }
   ```

## Documentation

- Update README.md for new features
- Add inline documentation for complex logic
- Include examples in TUTORIAL.md for major features
- Update SDK documentation for API changes

## Reporting Issues

- Use GitHub Issues for bug reports and feature requests
- Provide detailed reproduction steps for bugs
- Include version information and environment details
- Check for existing issues before creating new ones

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Help newcomers learn and contribute
- Focus on what is best for the community

## Questions?

- Join our [Discord community](https://discord.gg/belizechain)
- Ask in GitHub Discussions
- Email: dev@belizechain.io

Thank you for contributing to BelizeChain! 💎🇧🇿
