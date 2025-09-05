<template>
  <div class="payment-container">
    <div class="payment-card">
      <h2>WoPay - MetaMask支付</h2>

      <!-- 钱包连接状态 -->
      <div class="wallet-status">
        <div v-if="!isConnected" class="status-disconnected">
          <p>🦊 请连接您的MetaMask钱包</p>
          <button @click="connectWallet" class="connect-btn" :disabled="connecting">
            {{ connecting ? '连接中...' : '连接MetaMask' }}
          </button>
        </div>

        <div v-else class="status-connected">
          <p>✅ 钱包已连接</p>
          <p class="wallet-address">地址: {{ formatAddress(account) }}</p>
          <p class="wallet-balance">余额: {{ balance }} ETH</p>
        </div>
      </div>

      <!-- 支付表单 -->
      <div v-if="isConnected" class="payment-form">
        <h3>发起支付</h3>

        <div class="form-group">
          <label>收款地址:</label>
          <input v-model="paymentData.to" type="text" placeholder="0x..." class="form-input" />
        </div>

        <div class="form-group">
          <label>支付金额 (ETH):</label>
          <input v-model="paymentData.amount" type="number" step="0.001" placeholder="0.001" class="form-input" />
        </div>

        <div class="form-group">
          <label>备注 (可选):</label>
          <input v-model="paymentData.memo" type="text" placeholder="支付备注" class="form-input" />
        </div>

        <button @click="sendPayment" class="pay-btn" :disabled="!canPay || paying">
          {{ paying ? '支付中...' : '发起支付' }}
        </button>
      </div>

      <!-- 交易状态 -->
      <div v-if="transactionStatus" class="transaction-status">
        <div :class="['status-message', transactionStatus.type]">
          {{ transactionStatus.message }}
        </div>
        <div v-if="transactionStatus.hash" class="transaction-hash">
          <p>交易哈希:
            <a :href="getEtherscanUrl(transactionStatus.hash)" target="_blank">
              {{ formatHash(transactionStatus.hash) }}
            </a>
          </p>
        </div>
      </div>

      <!-- 交易历史 -->
      <div v-if="transactionHistory.length > 0" class="transaction-history">
        <h3>最近交易</h3>
        <div v-for="tx in transactionHistory" :key="tx.hash" class="history-item">
          <div class="tx-info">
            <span class="tx-amount">{{ tx.amount }} ETH</span>
            <span class="tx-to">→ {{ formatAddress(tx.to) }}</span>
          </div>
          <div class="tx-meta">
            <span class="tx-time">{{ formatTime(tx.timestamp) }}</span>
            <a :href="getEtherscanUrl(tx.hash)" target="_blank" class="tx-link">
              查看详情
            </a>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted, shallowRef, markRaw } from 'vue'
import { ethers } from 'ethers'

// 响应式数据
const isConnected = ref(false)
const connecting = ref(false) 
const paying = ref(false)
const account = ref('')
const balance = ref('0')
var provider = shallowRef(null)
const signer = shallowRef(null)

// 支付数据
const paymentData = reactive({
  to: '',
  amount: '',
  memo: ''
})

// 交易状态
const transactionStatus = ref(null)
const transactionHistory = ref([])

// 计算属性
const canPay = computed(() => {
  return paymentData.to &&
    paymentData.amount &&
    parseFloat(paymentData.amount) > 0 &&
    ethers.isAddress(paymentData.to)
})

// 获取MetaMask提供者
const getMetaMaskProvider = () => {
  if (typeof window.ethereum !== 'undefined') {
    provider.value = markRaw(new ethers.BrowserProvider(window.ethereum))
  }
}

// 连接MetaMask钱包
const connectWallet = async () => {
  getMetaMaskProvider()
  if (!provider.value) {
    showStatus('error', '请安装MetaMask钱包或确保MetaMask已启用')
    return
  }
  try {
    connecting.value = true
    // 请求账户访问权限
    const accounts = await window.ethereum.request({ method: 'eth_requestAccounts' });
    account.value = accounts[0];
    if (accounts.length === 0) {
      showStatus('error', '未获取到账户信息')
      return
    }

    // 等待一小段时间确保MetaMask完全初始化
    await new Promise(resolve => setTimeout(resolve, 100))
    
    // 创建signer - 在用户授权后
    signer.value = markRaw(await provider.value.getSigner())

    // 获取账户信息
    // account.value = await signer.value.getAddress() // 已经从accounts[0]获取
    await updateBalance()

    isConnected.value = true
    showStatus('success', 'MetaMask连接成功')

  } catch (error) {
    console.error('连接MetaMask失败:', error)
  } finally {
    connecting.value = false
  }
}

// 更新余额
const updateBalance = async () => {
  if (!provider.value || !account.value) return

  try {
    const balanceWei = await provider.value.getBalance(account.value)
    balance.value = ethers.formatEther(balanceWei)
  } catch (error) {
    console.error('获取余额失败:', error)
  }
}

// 发起支付
const sendPayment = async () => {
  if (!canPay.value) return
  
  // 确保有signer
  if (!signer.value) {
    try {
      signer.value = provider.value.getSigner()
    } catch (error) {
      showStatus('error', '无法创建签名器，请重新连接钱包')
      return
    }
  }

  try {
    paying.value = true
    showStatus('info', '正在发起交易...')

    // 构建交易
    const transaction = {
      to: paymentData.to,
      value: ethers.parseEther(paymentData.amount),
      data: paymentData.memo ? ethers.toUtf8Bytes(paymentData.memo) : '0x'
    }

    // 发送交易
    const tx = await signer.value.sendTransaction(transaction)

    showStatus('info', '交易已提交，等待确认...', tx.hash)

    // 等待交易确认
    const receipt = await tx.wait()

    if (receipt.status === 1) {
      showStatus('success', '支付成功！', tx.hash)

      // 添加到交易历史
      addToHistory({
        hash: tx.hash,
        to: paymentData.to,
        amount: paymentData.amount,
        timestamp: Date.now()
      })

      // 清空表单
      paymentData.to = ''
      paymentData.amount = ''
      paymentData.memo = ''

      // 更新余额
      await updateBalance()
    } else {
      showStatus('error', '交易失败')
    }

  } catch (error) {
    console.error('支付失败:', error)

    if (error.code === 'ACTION_REJECTED') {
      showStatus('error', '用户取消了交易')
    } else if (error.code === 'INSUFFICIENT_FUNDS') {
      showStatus('error', '余额不足')
    } else {
      showStatus('error', '支付失败: ' + error.message)
    }
  } finally {
    paying.value = false
  }
}

// 显示状态消息
const showStatus = (type, message, hash = null) => {
  transactionStatus.value = { type, message, hash }

  // 3秒后自动清除状态（除了成功状态）
  if (type !== 'success') {
    setTimeout(() => {
      transactionStatus.value = null
    }, 3000)
  }
}

// 添加到交易历史
const addToHistory = (transaction) => {
  transactionHistory.value.unshift(transaction)

  // 只保留最近10条记录
  if (transactionHistory.value.length > 10) {
    transactionHistory.value = transactionHistory.value.slice(0, 10)
  }

  // 保存到localStorage
  localStorage.setItem('wopay_transactions', JSON.stringify(transactionHistory.value))
}

// 处理账户变化
const handleAccountsChanged = (accounts) => {
  if (accounts.length === 0) {
    // 用户断开了钱包连接
    isConnected.value = false
    account.value = ''
    balance.value = '0'
    showStatus('info', '钱包已断开连接')
  } else {
    // 用户切换了账户
    account.value = accounts[0]
    updateBalance()
    showStatus('info', '账户已切换')
  }
}

// 处理网络变化
const handleChainChanged = () => {
  // 网络变化时重新加载页面
  window.location.reload()
}

// 格式化地址
const formatAddress = (address) => {
  if (!address) return ''
  return `${address.slice(0, 6)}...${address.slice(-4)}`
}

// 格式化哈希
const formatHash = (hash) => {
  return `${hash.slice(0, 10)}...${hash.slice(-8)}`
}

// 格式化时间
const formatTime = (timestamp) => {
  return new Date(timestamp).toLocaleString('zh-CN')
}

// 获取Etherscan链接
const getEtherscanUrl = (hash) => {
  // 这里假设是主网，实际使用时需要根据当前网络调整
  return `https://etherscan.io/tx/${hash}`
}

// 组件挂载时的初始化
onMounted(async () => {
  // 检查是否已经连接过MetaMask钱包
  getMetaMaskProvider()
  if (provider.value) {
    try {
      // 检查是否已有连接的账户
      const accounts = await window.ethereum.request({ method: 'eth_accounts' })
      if (accounts && accounts.length > 0) {
        // 自动重连
        await connectWallet()
      }
    } catch (error) {
      console.log('检查已连接账户失败:', error)
    }
  }
})

// 组件卸载时清理事件监听
import { onUnmounted } from 'vue'

onUnmounted(() => {
  if (window.ethereum) {
    window.ethereum.removeListener('accountsChanged', handleAccountsChanged)
    window.ethereum.removeListener('chainChanged', handleChainChanged)
  }
})
</script>

<style scoped>
.payment-container {
  max-width: 600px;
  margin: 0 auto;
  padding: 20px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

.payment-card {
  background: white;
  border-radius: 16px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
  padding: 32px;
  border: 1px solid #e5e7eb;
}

h2 {
  text-align: center;
  color: #1f2937;
  margin-bottom: 32px;
  font-size: 28px;
  font-weight: 700;
}

h3 {
  color: #374151;
  margin-bottom: 20px;
  font-size: 20px;
  font-weight: 600;
}

.wallet-status {
  margin-bottom: 32px;
  padding: 20px;
  border-radius: 12px;
  text-align: center;
}

.status-disconnected {
  background: #fef3c7;
  border: 1px solid #f59e0b;
}

.status-connected {
  background: #d1fae5;
  border: 1px solid #10b981;
}

.wallet-address,
.wallet-balance {
  font-size: 14px;
  color: #6b7280;
  margin: 8px 0;
  word-break: break-all;
}

.connect-btn,
.pay-btn {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  border: none;
  padding: 12px 24px;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  width: 100%;
  margin-top: 16px;
}

.connect-btn:hover,
.pay-btn:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

.connect-btn:disabled,
.pay-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
}

.payment-form {
  margin-bottom: 32px;
}

.form-group {
  margin-bottom: 20px;
}

.form-group label {
  display: block;
  margin-bottom: 8px;
  color: #374151;
  font-weight: 500;
}

.form-input {
  width: 100%;
  padding: 12px 16px;
  border: 2px solid #e5e7eb;
  border-radius: 8px;
  font-size: 16px;
  transition: border-color 0.2s;
  box-sizing: border-box;
}

.form-input:focus {
  outline: none;
  border-color: #667eea;
}

.transaction-status {
  margin-bottom: 24px;
  padding: 16px;
  border-radius: 8px;
}

.status-message {
  font-weight: 500;
  margin-bottom: 8px;
}

.status-message.success {
  color: #065f46;
  background: #d1fae5;
  border: 1px solid #10b981;
  padding: 12px;
  border-radius: 8px;
}

.status-message.error {
  color: #991b1b;
  background: #fee2e2;
  border: 1px solid #ef4444;
  padding: 12px;
  border-radius: 8px;
}

.status-message.info {
  color: #1e40af;
  background: #dbeafe;
  border: 1px solid #3b82f6;
  padding: 12px;
  border-radius: 8px;
}

.transaction-hash {
  font-size: 14px;
  color: #6b7280;
}

.transaction-hash a {
  color: #667eea;
  text-decoration: none;
}

.transaction-hash a:hover {
  text-decoration: underline;
}

.transaction-history {
  border-top: 1px solid #e5e7eb;
  padding-top: 24px;
}

.history-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid #f3f4f6;
}

.history-item:last-child {
  border-bottom: none;
}

.tx-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.tx-amount {
  font-weight: 600;
  color: #1f2937;
}

.tx-to {
  font-size: 14px;
  color: #6b7280;
}

.tx-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
}

.tx-time {
  font-size: 12px;
  color: #9ca3af;
}

.tx-link {
  font-size: 12px;
  color: #667eea;
  text-decoration: none;
}

.tx-link:hover {
  text-decoration: underline;
}

@media (max-width: 640px) {
  .payment-container {
    padding: 16px;
  }

  .payment-card {
    padding: 24px;
  }

  .history-item {
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }

  .tx-meta {
    align-items: flex-start;
  }
}
</style>
