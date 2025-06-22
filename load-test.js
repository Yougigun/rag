import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
    stages: [
        { duration: '10s', target: 5 },   // Ramp up to 5 users over 10 seconds
        { duration: '30s', target: 5 },   // Stay at 5 users for 30 seconds
        { duration: '10s', target: 0 },   // Ramp down to 0 users over 10 seconds
    ],
    thresholds: {
        http_req_duration: ['p(95)<500'], // 95% of requests must complete below 500ms
        http_req_failed: ['rate<0.1'],    // Error rate must be below 10%
    },
};

const BASE_URL = 'http://localhost:3000';

export default function () {
    // Test health check endpoint
    const healthResponse = http.get(`${BASE_URL}/api/v1/health`);

    check(healthResponse, {
        'health check status is 200': (r) => r.status === 200,
        'health check has correct status': (r) => {
            const body = JSON.parse(r.body);
            return body.status === 'ok' && body.service === 'rag-api';
        },
    });

    // Test query endpoint
    const queryPayload = JSON.stringify({
        query: 'What is the meaning of life?',
        system_prompt: 'You are a helpful assistant',
        user_prompt: 'Please answer the following question',
        json_mode: false,
    });

    const queryParams = {
        headers: {
            'Content-Type': 'application/json',
        },
    };

    const queryResponse = http.post(`${BASE_URL}/api/v1/query`, queryPayload, queryParams);

    check(queryResponse, {
        'query status is 200': (r) => r.status === 200,
        'query has correct response format': (r) => {
            const body = JSON.parse(r.body);
            return body.status === 'ok' &&
                body.message === 'Query processed successfully' &&
                body.query === 'What is the meaning of life?';
        },
    });

    // Add a small delay between iterations
    sleep(1);
}

export function handleSummary(data) {
    return {
        'stdout': textSummary(data, { indent: ' ', enableColors: true }),
    };
}

function textSummary(data, options = {}) {
    const indent = options.indent || '';
    const enableColors = options.enableColors || false;

    let output = '';

    if (enableColors) {
        output += `\n${indent}\x1b[36m=== K6 Load Test Summary ===\x1b[0m\n`;
    } else {
        output += `\n${indent}=== K6 Load Test Summary ===\n`;
    }

    output += `${indent}Total Requests: ${data.metrics.http_reqs.values.count}\n`;
    output += `${indent}Failed Requests: ${data.metrics.http_req_failed.values.passes || 0}\n`;
    output += `${indent}Average Response Time: ${data.metrics.http_req_duration.values.avg.toFixed(2)}ms\n`;
    output += `${indent}95th Percentile: ${data.metrics.http_req_duration.values['p(95)'].toFixed(2)}ms\n`;
    output += `${indent}Test Duration: ${(data.state.testRunDurationMs / 1000).toFixed(2)}s\n`;

    return output;
} 