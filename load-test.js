import http from 'k6/http';
import { check } from 'k6';

export const options = {
    vus: 1,        // Single virtual user
    iterations: 1, // Run only once
};

const BASE_URL = 'http://localhost:3000';

export default function () {
    // Test health check endpoint
    const healthResponse = http.get(`${BASE_URL}/api/v1/health`);

    const healthCheckPassed = check(healthResponse, {
        'health check status is 200': (r) => r.status === 200,
        'health check returns OK': (r) => {
            try {
                const body = JSON.parse(r.body);
                return body.status === 'ok' && body.service === 'rag-api';
            } catch (e) {
                console.log('Failed to parse health check response:', r.body);
                return false;
            }
        },
    });

    if (healthCheckPassed) {
        console.log('✓ RAG API health check passed - service is running correctly');
    } else {
        console.log('✗ RAG API health check failed');
        console.log('Response status:', healthResponse.status);
        console.log('Response body:', healthResponse.body);
    }
} 