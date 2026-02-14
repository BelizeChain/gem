/**
 * Basic SDK validation tests
 * Ensures all modules load correctly and have expected exports
 */

const assert = require('assert');
const path = require('path');

// Test suite runner
class TestRunner {
  constructor() {
    this.passed = 0;
    this.failed = 0;
    this.tests = [];
  }

  test(name, fn) {
    this.tests.push({ name, fn });
  }

  async run() {
    console.log('🧪 Running SDK Tests\n');

    for (const { name, fn } of this.tests) {
      try {
        await fn();
        console.log(`✅ ${name}`);
        this.passed++;
      } catch (error) {
        console.error(`❌ ${name}`);
        console.error(`   ${error.message}`);
        this.failed++;
      }
    }

    console.log(`\n📊 Results: ${this.passed} passed, ${this.failed} failed`);

    if (this.failed > 0) {
      process.exit(1);
    }
  }
}

const runner = new TestRunner();

// Test: Main SDK module loads
runner.test('index.js exports GemSDK', () => {
  const sdk = require('../index.js');
  assert(sdk, 'index.js should export something');
  assert(
    typeof sdk === 'object' || typeof sdk === 'function',
    'index.js should export object or function'
  );
});

// Test: BelizeX module loads
runner.test('belizex.js exports BelizeXSDK', () => {
  const BelizeXSDK = require('../belizex.js');
  assert(BelizeXSDK, 'belizex.js should export BelizeXSDK');
  assert(typeof BelizeXSDK === 'function', 'BelizeXSDK should be a constructor');
});

// Test: MeshNetwork module loads
runner.test('meshNetwork.js exports MeshNetworkSDK', () => {
  const MeshNetworkSDK = require('../meshNetwork.js');
  assert(MeshNetworkSDK, 'meshNetwork.js should export MeshNetworkSDK');
  assert(typeof MeshNetworkSDK === 'function', 'MeshNetworkSDK should be a constructor');
});

// Test: Privacy module loads
runner.test('privacy.js exports PrivacySDK', () => {
  const PrivacySDK = require('../privacy.js');
  assert(PrivacySDK, 'privacy.js should export PrivacySDK');
  assert(typeof PrivacySDK === 'function', 'PrivacySDK should be a constructor');
});

// Test: Contract ABIs exist
runner.test('Contract ABIs are available', () => {
  const fs = require('fs');
  const contracts = ['dalla.json', 'belinft.json', 'dao.json', 'faucet.json'];

  for (const contract of contracts) {
    const contractPath = path.join(__dirname, '..', 'contracts', contract);
    assert(fs.existsSync(contractPath), `${contract} should exist`);

    const abi = require(contractPath);
    assert(abi.source, 'ABI should have source field');
    assert(abi.spec, 'ABI should have spec field');
  }
});

// Test: Package.json is valid
runner.test('package.json has correct metadata', () => {
  const pkg = require('../package.json');
  assert.strictEqual(
    pkg.name,
    '@belizechain/gem-sdk',
    'Package name should be @belizechain/gem-sdk'
  );
  assert.strictEqual(pkg.version, '1.3.0', 'Version should be 1.3.0');
  assert.strictEqual(pkg.main, 'index.js', 'Main entry should be index.js');
  assert.strictEqual(pkg.types, 'index.d.ts', 'Types entry should be index.d.ts');
});

// Test: TypeScript definitions exist
runner.test('TypeScript definitions are present', () => {
  const fs = require('fs');
  const dtsFiles = ['index.d.ts', 'belizex.d.ts', 'meshNetwork.d.ts', 'privacy.d.ts'];

  for (const file of dtsFiles) {
    const filePath = path.join(__dirname, '..', file);
    assert(fs.existsSync(filePath), `${file} should exist`);
  }
});

// Test: Examples directory exists
runner.test('Examples directory has sample files', () => {
  const fs = require('fs');
  const examplesDir = path.join(__dirname, '..', 'examples');
  assert(fs.existsSync(examplesDir), 'examples/ directory should exist');

  const examples = fs.readdirSync(examplesDir);
  assert(examples.length > 0, 'examples/ should contain files');
});

// Test: README exists
runner.test('README.md exists and has content', () => {
  const fs = require('fs');
  const readmePath = path.join(__dirname, '..', 'README.md');
  assert(fs.existsSync(readmePath), 'README.md should exist');

  const content = fs.readFileSync(readmePath, 'utf8');
  assert(content.length > 1000, 'README should have substantial content');
  assert(content.includes('BelizeChain'), 'README should mention BelizeChain');
});

// Run all tests
runner.run().catch((error) => {
  console.error('Test runner error:', error);
  process.exit(1);
});
