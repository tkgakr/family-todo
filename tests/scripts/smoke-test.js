#!/usr/bin/env node

import axios from 'axios'

const API_ENDPOINT = process.env.API_ENDPOINT
const USER_POOL_ID = process.env.USER_POOL_ID
const USER_POOL_CLIENT_ID = process.env.USER_POOL_CLIENT_ID

if (!API_ENDPOINT) {
  console.error('ERROR: API_ENDPOINT environment variable is required')
  process.exit(1)
}

console.log('🔥 Running smoke tests...')
console.log(`API Endpoint: ${API_ENDPOINT}`)

const delay = (ms) => new Promise(resolve => setTimeout(resolve, ms))

async function runSmokeTests() {
  const results = []
  
  try {
    // Test 1: Health check (if implemented)
    console.log('\n🏥 Testing health check...')
    try {
      const response = await axios.get(`${API_ENDPOINT}/health`, {
        timeout: 5000,
        headers: {
          'X-Family-Id': 'smoke-test-family',
          'X-User-Id': 'smoke-test-user',
        },
      })
      results.push({ test: 'Health Check', status: 'PASS', details: `Status: ${response.status}` })
      console.log('✅ Health check passed')
    } catch (error) {
      if (error.response?.status === 404) {
        results.push({ test: 'Health Check', status: 'SKIP', details: 'Health endpoint not implemented' })
        console.log('⏭️  Health check skipped (not implemented)')
      } else {
        results.push({ test: 'Health Check', status: 'FAIL', details: error.message })
        console.log('❌ Health check failed:', error.message)
      }
    }

    // Test 2: Get todos (should work without auth for testing)
    console.log('\n📋 Testing get todos endpoint...')
    try {
      const response = await axios.get(`${API_ENDPOINT}/todos`, {
        timeout: 10000,
        headers: {
          'X-Family-Id': 'smoke-test-family',
          'X-User-Id': 'smoke-test-user',
        },
      })
      
      if (response.status === 200) {
        const data = response.data
        if (data && typeof data.todos === 'object') {
          results.push({ test: 'Get Todos', status: 'PASS', details: `Returned ${data.todos.length} todos` })
          console.log('✅ Get todos endpoint working')
        } else {
          results.push({ test: 'Get Todos', status: 'FAIL', details: 'Invalid response format' })
          console.log('❌ Get todos returned invalid format')
        }
      } else {
        results.push({ test: 'Get Todos', status: 'FAIL', details: `Unexpected status: ${response.status}` })
        console.log(`❌ Get todos returned status: ${response.status}`)
      }
    } catch (error) {
      if (error.response?.status === 401) {
        results.push({ test: 'Get Todos', status: 'EXPECTED', details: 'Authentication required (expected)' })
        console.log('✅ Get todos requires authentication (expected)')
      } else {
        results.push({ test: 'Get Todos', status: 'FAIL', details: error.message })
        console.log('❌ Get todos failed:', error.message)
      }
    }

    // Test 3: CORS preflight
    console.log('\n🌐 Testing CORS preflight...')
    try {
      const response = await axios.options(`${API_ENDPOINT}/todos`, {
        headers: {
          'Access-Control-Request-Method': 'GET',
          'Access-Control-Request-Headers': 'Content-Type,Authorization',
        },
        timeout: 5000,
      })
      
      const corsHeaders = response.headers['access-control-allow-origin']
      if (corsHeaders) {
        results.push({ test: 'CORS Preflight', status: 'PASS', details: `CORS headers present` })
        console.log('✅ CORS preflight working')
      } else {
        results.push({ test: 'CORS Preflight', status: 'FAIL', details: 'No CORS headers found' })
        console.log('❌ CORS preflight failed - no headers')
      }
    } catch (error) {
      results.push({ test: 'CORS Preflight', status: 'FAIL', details: error.message })
      console.log('❌ CORS preflight failed:', error.message)
    }

    // Test 4: Invalid endpoint (should return 404)
    console.log('\n🚫 Testing invalid endpoint...')
    try {
      await axios.get(`${API_ENDPOINT}/invalid-endpoint`, {
        timeout: 5000,
        headers: {
          'X-Family-Id': 'smoke-test-family',
          'X-User-Id': 'smoke-test-user',
        },
      })
      results.push({ test: 'Invalid Endpoint', status: 'FAIL', details: 'Should have returned 404' })
      console.log('❌ Invalid endpoint should return 404')
    } catch (error) {
      if (error.response?.status === 404) {
        results.push({ test: 'Invalid Endpoint', status: 'PASS', details: 'Correctly returned 404' })
        console.log('✅ Invalid endpoint correctly returns 404')
      } else {
        results.push({ test: 'Invalid Endpoint', status: 'FAIL', details: `Expected 404, got ${error.response?.status || 'network error'}` })
        console.log(`❌ Invalid endpoint returned ${error.response?.status || 'network error'}`)
      }
    }

    // Test 5: API response time
    console.log('\n⚡ Testing API response time...')
    try {
      const startTime = Date.now()
      await axios.get(`${API_ENDPOINT}/todos`, {
        timeout: 10000,
        headers: {
          'X-Family-Id': 'smoke-test-family',
          'X-User-Id': 'smoke-test-user',
        },
      })
      const responseTime = Date.now() - startTime
      
      if (responseTime < 5000) {
        results.push({ test: 'Response Time', status: 'PASS', details: `${responseTime}ms` })
        console.log(`✅ API response time: ${responseTime}ms`)
      } else {
        results.push({ test: 'Response Time', status: 'WARN', details: `${responseTime}ms (slow)` })
        console.log(`⚠️  API response time: ${responseTime}ms (slow)`)
      }
    } catch (error) {
      results.push({ test: 'Response Time', status: 'FAIL', details: error.message })
      console.log('❌ Response time test failed:', error.message)
    }

  } catch (error) {
    console.error('💥 Smoke tests failed with unexpected error:', error.message)
    process.exit(1)
  }

  // Summary
  console.log('\n📊 Smoke Test Results:')
  console.log('═══════════════════════════════════════')
  
  let passed = 0
  let failed = 0
  let skipped = 0
  let warnings = 0
  
  results.forEach(result => {
    const status = result.status === 'PASS' ? '✅ PASS' : 
                   result.status === 'FAIL' ? '❌ FAIL' : 
                   result.status === 'SKIP' ? '⏭️  SKIP' :
                   result.status === 'EXPECTED' ? '✅ EXPECTED' :
                   result.status === 'WARN' ? '⚠️  WARN' : '❓ UNKNOWN'
    
    console.log(`${status} | ${result.test.padEnd(20)} | ${result.details}`)
    
    if (result.status === 'PASS' || result.status === 'EXPECTED') passed++
    else if (result.status === 'FAIL') failed++
    else if (result.status === 'SKIP') skipped++
    else if (result.status === 'WARN') warnings++
  })
  
  console.log('═══════════════════════════════════════')
  console.log(`Total: ${results.length} | Passed: ${passed} | Failed: ${failed} | Skipped: ${skipped} | Warnings: ${warnings}`)
  
  if (failed > 0) {
    console.log('\n💥 Some smoke tests failed!')
    process.exit(1)
  } else if (warnings > 0) {
    console.log('\n⚠️  Smoke tests completed with warnings')
    process.exit(0)
  } else {
    console.log('\n🎉 All smoke tests passed!')
    process.exit(0)
  }
}

runSmokeTests().catch(error => {
  console.error('💥 Smoke tests crashed:', error.message)
  process.exit(1)
})