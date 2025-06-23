#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}🧪 RAG System Test Suite${NC}"
echo -e "${BLUE}========================${NC}\n"

# Check if k6 is installed
if ! command -v k6 &> /dev/null; then
    echo -e "${RED}❌ k6 is not installed. Please install k6 first:${NC}"
    echo "   macOS: brew install k6"
    echo "   Linux: https://k6.io/docs/getting-started/installation/"
    exit 1
fi

# Check if services are running
echo -e "${BLUE}🔍 Checking if services are running...${NC}"
if ! curl -s http://localhost:3000/api/v1/health > /dev/null 2>&1; then
    echo -e "${RED}❌ RAG API is not running on localhost:3000${NC}"
    echo -e "${YELLOW}💡 Run 'make run' to start all services${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Services are running${NC}\n"

# Run smoke test first
echo -e "${BLUE}🔥 Running smoke test...${NC}"
if k6 run tests/smoke-test.js; then
    echo -e "${GREEN}✅ Smoke test passed${NC}\n"
else
    echo -e "${RED}❌ Smoke test failed${NC}"
    exit 1
fi

# Run comprehensive API test
echo -e "${BLUE}🧪 Running comprehensive API test...${NC}"
if k6 run tests/api-test.js; then
    echo -e "${GREEN}✅ API test passed${NC}\n"
else
    echo -e "${RED}❌ API test failed${NC}"
    exit 1
fi

# Ask user if they want to run load test
echo -e "${YELLOW}🚀 Do you want to run the load test? (y/N)${NC}"
read -r response
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    echo -e "${BLUE}📈 Running load test...${NC}"
    if k6 run tests/load-test.js; then
        echo -e "${GREEN}✅ Load test completed${NC}"
        if [ -f "tests/load-test-results.json" ]; then
            echo -e "${BLUE}📊 Load test results saved to tests/load-test-results.json${NC}"
        fi
    else
        echo -e "${RED}❌ Load test failed${NC}"
        exit 1
    fi
else
    echo -e "${BLUE}ℹ️  Skipping load test${NC}"
fi

echo -e "\n${GREEN}🎉 All tests completed successfully!${NC}"
echo -e "${BLUE}📝 Test Summary:${NC}"
echo -e "   ✅ Smoke test - Basic service health"
echo -e "   ✅ API test - Full CRUD operations"
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    echo -e "   ✅ Load test - Performance under load"
fi