import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

const errorRate = new Rate('errors');
const todoCreationTrend = new Trend('todo_creation_duration');
const todoRetrievalTrend = new Trend('todo_retrieval_duration');

export const options = {
  stages: [
    { duration: '2m', target: 20 },   // Ramp up to 20 users
    { duration: '5m', target: 20 },   // Stay at 20 users
    { duration: '2m', target: 50 },   // Ramp up to 50 users
    { duration: '5m', target: 50 },   // Stay at 50 users
    { duration: '2m', target: 0 },    // Ramp down to 0 users
  ],
  thresholds: {
    http_req_duration: ['p(95)<2000'], // 95% of requests should complete within 2s
    http_req_failed: ['rate<0.05'],    // Error rate should be less than 5%
    errors: ['rate<0.05'],             // Custom error rate should be less than 5%
    todo_creation_duration: ['p(95)<1000'], // Todo creation should be fast
    todo_retrieval_duration: ['p(95)<500'], // Todo retrieval should be very fast
  },
};

const BASE_URL = __ENV.API_ENDPOINT || 'https://your-api.execute-api.region.amazonaws.com/prod';
const AUTH_TOKEN = __ENV.AUTH_TOKEN || '';

export default function () {
  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${AUTH_TOKEN}`,
      'X-Family-Id': `load-test-family-${__VU}`,
      'X-User-Id': `load-test-user-${__VU}`,
    },
    timeout: '30s',
  };

  // Test 1: Get todos (read operation)
  const getTodosResponse = http.get(`${BASE_URL}/todos`, params);
  
  const getTodosCheck = check(getTodosResponse, {
    'get todos status is 200 or 401': (r) => r.status === 200 || r.status === 401,
    'get todos response time < 2s': (r) => r.timings.duration < 2000,
  });
  
  if (getTodosResponse.status === 200) {
    todoRetrievalTrend.add(getTodosResponse.timings.duration);
  }
  
  errorRate.add(!getTodosCheck);

  sleep(1);

  // Test 2: Create todo (write operation) - only if we have auth
  if (AUTH_TOKEN && getTodosResponse.status === 200) {
    const todoPayload = JSON.stringify({
      title: `Load Test Todo ${Date.now()}-${__VU}`,
      description: `This is a load test todo created by virtual user ${__VU}`,
      tags: ['load-test', 'performance'],
    });

    const createTodoResponse = http.post(`${BASE_URL}/todos`, todoPayload, params);
    
    const createTodoCheck = check(createTodoResponse, {
      'create todo status is 201': (r) => r.status === 201,
      'create todo response time < 3s': (r) => r.timings.duration < 3000,
      'create todo returns todo object': (r) => {
        try {
          const todo = JSON.parse(r.body);
          return todo && todo.id && todo.title;
        } catch {
          return false;
        }
      },
    });
    
    if (createTodoResponse.status === 201) {
      todoCreationTrend.add(createTodoResponse.timings.duration);
      
      try {
        const createdTodo = JSON.parse(createTodoResponse.body);
        
        // Test 3: Get specific todo
        const getTodoResponse = http.get(`${BASE_URL}/todos/${createdTodo.id}`, params);
        
        check(getTodoResponse, {
          'get specific todo status is 200': (r) => r.status === 200,
          'get specific todo response time < 1s': (r) => r.timings.duration < 1000,
        });
        
        // Test 4: Complete todo
        const completeTodoResponse = http.post(`${BASE_URL}/todos/${createdTodo.id}/complete`, null, params);
        
        check(completeTodoResponse, {
          'complete todo status is 200': (r) => r.status === 200,
          'complete todo response time < 2s': (r) => r.timings.duration < 2000,
        });
        
        sleep(0.5);
        
        // Test 5: Delete todo
        const deleteTodoResponse = http.del(`${BASE_URL}/todos/${createdTodo.id}`, null, params);
        
        check(deleteTodoResponse, {
          'delete todo status is 204': (r) => r.status === 204,
          'delete todo response time < 2s': (r) => r.timings.duration < 2000,
        });
        
      } catch (error) {
        console.warn('Failed to parse created todo response:', error);
        errorRate.add(1);
      }
    } else {
      errorRate.add(1);
    }
    
    errorRate.add(!createTodoCheck);
  }

  // Test 6: Get family members (if implemented)
  const getFamilyResponse = http.get(`${BASE_URL}/family/members`, params);
  
  check(getFamilyResponse, {
    'get family members status is 200 or 401 or 404': (r) => 
      r.status === 200 || r.status === 401 || r.status === 404,
    'get family members response time < 1s': (r) => r.timings.duration < 1000,
  });

  // Random sleep between 0.5 and 2 seconds to simulate realistic user behavior
  sleep(Math.random() * 1.5 + 0.5);
}

export function handleSummary(data) {
  return {
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
    'test-results/load-test-summary.json': JSON.stringify(data, null, 2),
  };
}

function textSummary(data, options = {}) {
  const indent = options.indent || '';
  const summaryData = data.metrics;
  
  let summary = '\n' + indent + '📊 Load Test Summary\n';
  summary += indent + '═══════════════════════════════════════\n';
  
  // Key metrics
  if (summaryData.http_req_duration) {
    summary += indent + `🕒 HTTP Request Duration:\n`;
    summary += indent + `   avg: ${summaryData.http_req_duration.values.avg.toFixed(2)}ms\n`;
    summary += indent + `   p95: ${summaryData.http_req_duration.values.p95.toFixed(2)}ms\n`;
    summary += indent + `   p99: ${summaryData.http_req_duration.values.p99.toFixed(2)}ms\n`;
  }
  
  if (summaryData.http_req_failed) {
    const failedRate = summaryData.http_req_failed.values.rate * 100;
    summary += indent + `❌ HTTP Request Failed Rate: ${failedRate.toFixed(2)}%\n`;
  }
  
  if (summaryData.http_reqs) {
    summary += indent + `📈 Total HTTP Requests: ${summaryData.http_reqs.values.count}\n`;
  }
  
  if (summaryData.vus) {
    summary += indent + `👥 Peak Virtual Users: ${summaryData.vus.values.max}\n`;
  }
  
  if (summaryData.todo_creation_duration) {
    summary += indent + `➕ Todo Creation Duration:\n`;
    summary += indent + `   avg: ${summaryData.todo_creation_duration.values.avg.toFixed(2)}ms\n`;
    summary += indent + `   p95: ${summaryData.todo_creation_duration.values.p95.toFixed(2)}ms\n`;
  }
  
  if (summaryData.todo_retrieval_duration) {
    summary += indent + `📋 Todo Retrieval Duration:\n`;
    summary += indent + `   avg: ${summaryData.todo_retrieval_duration.values.avg.toFixed(2)}ms\n`;
    summary += indent + `   p95: ${summaryData.todo_retrieval_duration.values.p95.toFixed(2)}ms\n`;
  }
  
  summary += indent + '═══════════════════════════════════════\n\n';
  
  return summary;
}