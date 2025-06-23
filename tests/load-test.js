import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
    stages: [
        { duration: '30s', target: 5 },   // Ramp up to 5 users
        { duration: '1m', target: 10 },   // Stay at 10 users for 1 minute
        { duration: '30s', target: 0 },   // Ramp down to 0 users
    ],
    thresholds: {
        http_req_duration: ['p(95)<1000'], // 95% of requests must be below 1s
        http_req_failed: ['rate<0.05'],    // Error rate must be below 5%
    },
};

const BASE_URL = 'http://localhost:3000';

export default function () {
    // Test health endpoint
    let response = http.get(`${BASE_URL}/api/v1/health`);
    check(response, {
        'health check status is 200': (r) => r.status === 200,
        'health check response time < 500ms': (r) => r.timings.duration < 500,
    });

    sleep(1);

    // Test create embedding task
    const createPayload = {
        file_name: `load-test-${__VU}-${__ITER}.txt`
    };

    response = http.post(
        `${BASE_URL}/api/v1/embedding-tasks`,
        JSON.stringify(createPayload),
        {
            headers: {
                'Content-Type': 'application/json',
            },
        }
    );

    let taskId;
    check(response, {
        'create task status is 201': (r) => r.status === 201,
        'create task response time < 1000ms': (r) => r.timings.duration < 1000,
        'create task returns valid id': (r) => {
            try {
                const body = JSON.parse(r.body);
                taskId = body.id;
                return body.id > 0;
            } catch (e) {
                return false;
            }
        },
    });

    sleep(1);

    // Test list tasks
    response = http.get(`${BASE_URL}/api/v1/embedding-tasks`);
    check(response, {
        'list tasks status is 200': (r) => r.status === 200,
        'list tasks response time < 500ms': (r) => r.timings.duration < 500,
    });

    sleep(1);

    // Test get specific task
    if (taskId) {
        response = http.get(`${BASE_URL}/api/v1/embedding-tasks/${taskId}`);
        check(response, {
            'get task status is 200': (r) => r.status === 200,
            'get task response time < 500ms': (r) => r.timings.duration < 500,
        });

        sleep(1);

        // Clean up - delete the task
        response = http.del(`${BASE_URL}/api/v1/embedding-tasks/${taskId}`);
        check(response, {
            'delete task status is 204': (r) => r.status === 204,
        });
    }

    sleep(2);
}

export function handleSummary(data) {
    return {
        'stdout': textSummary(data, { indent: ' ', enableColors: true }),
        'tests/load-test-results.json': JSON.stringify(data),
    };
}