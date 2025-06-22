# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a RAG (Retrieval-Augmented Generation) system that combines local text file retrieval with OpenAI integration for contextual responses. The system ingests local text files into a vector database, performs similarity searches, and uses OpenAI GPT-4o to generate responses with injected context.

## Architecture Components

### Backend System
- **Vector Database**: Stores embeddings of local text file chunks
- **Similarity Search Engine**: Returns top 5 most relevant files based on query
- **OpenAI Integration**: GPT-4o with context injection from retrieved files
- **API Integration**: Fetches data from external endpoints to include in context
- **Response Formatting**: Supports both JSON and plain text output modes

### Frontend Interface
- **QueryInput**: Text area for user queries
- **ResponseDisplay**: Shows AI responses
- **Optional Components**: FileList (retrieved files), JsonToggle (response format)

## Development Approach

### Estimated Development Time
- Backend: ~2 hours
- Frontend: ~30 minutes  
- TDD Implementation: ~30 minutes
- Test Data Setup: ~30 minutes

### Key Implementation Requirements
- Support for both JSON and plain text response modes
- Vector embeddings for text file chunks with similarity scoring
- System prompt + user prompt support
- Context construction from retrieved files and API data
- Schema validation for JSON responses

### Input/Output Structure
The system accepts queries with optional system prompts, user prompts, JSON mode flags, and API endpoints. It returns either structured JSON with recommendations/sources or plain text responses based on the mode selected.

## File Organization
- `rag.md`: Product Requirements Document with detailed specifications
- Implementation files should follow the component structure outlined in the PRD