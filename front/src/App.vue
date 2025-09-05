<script setup>
import MetaMaskPayment from './components/MetaMaskPayment.vue'
</script>

<template>
  <div id="app">
    <header class="app-header">
      <div class="header-content">
        <h1>🚀 WoPay</h1>
        <p>基于区块链的去中心化支付平台</p>
      </div>
    </header>
    
    <main class="app-main">
      <MetaMaskPayment />
    </main>
    
    <footer class="app-footer">
      <p>&copy; 2024 WoPay. 安全 · 快速 · 去中心化</p>
    </footer>
  </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  min-height: 100vh;
}

#app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.app-header {
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  border-bottom: 1px solid rgba(255, 255, 255, 0.2);
  padding: 2rem 0;
}

.header-content {
  max-width: 1200px;
  margin: 0 auto;
  text-align: center;
  color: white;
  padding: 0 20px;
}

.header-content h1 {
  font-size: 3rem;
  font-weight: 700;
  margin-bottom: 0.5rem;
  text-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
}

.header-content p {
  font-size: 1.2rem;
  opacity: 0.9;
  font-weight: 300;
}

.app-main {
  flex: 1;
  padding: 3rem 0;
}

.app-footer {
  background: rgba(0, 0, 0, 0.2);
  color: white;
  text-align: center;
  padding: 1.5rem 0;
  font-size: 0.9rem;
  opacity: 0.8;
}

@media (max-width: 768px) {
  .header-content h1 {
    font-size: 2.5rem;
  }
  
  .header-content p {
    font-size: 1rem;
  }
  
  .app-main {
    padding: 2rem 0;
  }
}
</style>
