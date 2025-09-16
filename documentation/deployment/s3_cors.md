# S3 CORS Configuration Guide

## Problem Analysis

When attempting to upload files directly to S3 using presigned URLs, you may encounter the following error:

\`\`\`
Access to XMLHttpRequest at 'https://your-bucket.s3.region.amazonaws.com/...' from origin 'http://localhost:5001' 
has been blocked by CORS policy: Response to preflight request doesn't pass access control check: 
No 'Access-Control-Allow-Origin' header is present on the requested resource.
\`\`\`

This error occurs because:

1. When your browser attempts to upload a file directly to S3 using a presigned URL, it first sends a preflight OPTIONS request to check if cross-origin requests are allowed.
2. By default, S3 buckets don't allow cross-origin requests from web applications unless explicitly configured.
3. The browser blocks the request due to the Same-Origin Policy security feature.

## Solution: Configure CORS on your S3 Bucket

### Step 1: Create a CORS Configuration File

Create a file named \`cors-config.json\` with the following content:

```json
{
  "CORSRules": [
    {
      "AllowedOrigins": ["http://localhost:5001", "https://your-production-domain.com"],
      "AllowedHeaders": ["*"],
      "AllowedMethods": ["GET", "PUT", "POST", "DELETE", "HEAD"],
      "MaxAgeSeconds": 3000,
      "ExposeHeaders": ["ETag"]
    }
  ]
}
```

> Note: Replace \`https://your-production-domain.com\` with your actual production domain. You can include multiple domains in the \`AllowedOrigins\` array.

### Step 2: Apply the CORS Configuration to Your S3 Bucket

Use the AWS CLI to apply the configuration:

```bash
aws s3api put-bucket-cors --bucket paloit-cve --cors-configuration file://cors-config.json
```

Replace \`paloit-cve\` with your actual S3 bucket name.

### Step 3: Verify the CORS Configuration

You can verify that your CORS configuration has been applied by running:

```bash
aws s3api get-bucket-cors --bucket paloit-cve
```

## Technical Details

### Why CORS is Necessary

Cross-Origin Resource Sharing (CORS) is a security feature implemented by web browsers that restricts web pages from making requests to a different domain than the one that served the original page. When your web application running on one domain (e.g., localhost:5001) attempts to directly upload files to Amazon S3 (a different domain), the browser applies CORS restrictions.

### How Presigned URLs Work with CORS

1. Our backend generates a presigned URL for S3 using AWS SDK.
2. The frontend receives this URL and attempts to make a direct PUT request to S3.
3. Before making the actual PUT request, the browser sends a preflight OPTIONS request to check if the cross-origin request is allowed.
4. Without proper CORS configuration, S3 rejects this preflight request, and the browser blocks the subsequent PUT request.

### Alternative Approaches

If you cannot modify the S3 bucket CORS settings, consider these alternatives:

1. **Proxy Through Backend**: Upload files to your backend server first, then have your server upload to S3.
2. **Use S3 Transfer Acceleration**: For improved upload performance in production environments.
3. **Use AWS Amplify or AWS SDK for Browser**: These libraries include built-in retry logic and better error handling for S3 operations.

## Common Issues and Troubleshooting

1. **Multiple Origins**: Make sure all domains that will access the S3 bucket are listed in the \`AllowedOrigins\` array.
2. **HTTP vs HTTPS**: If your application runs on both HTTP and HTTPS, include both protocols in your CORS configuration.
3. **Credentials**: If using \`withCredentials: true\` in your XHR or fetch requests, your CORS configuration needs \`AllowCredentials: true\`.
4. **Cache Issues**: CORS responses are often cached by browsers. Clear your browser cache if testing changes.

## Example Code for Upload

```javascript
// Using XMLHttpRequest for maximum compatibility with S3
const uploadToS3 = (presignedUrl, file, contentType) => {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    
    xhr.open('PUT', presignedUrl);
    xhr.setRequestHeader('Content-Type', contentType);
    xhr.withCredentials = false; // Important for S3 requests
    
    xhr.onload = function() {
      if (xhr.status >= 200 && xhr.status < 300) {
        resolve({
          ok: true,
          status: xhr.status
        });
      } else {
        reject(new Error(\`Upload failed with status: \${xhr.status}\`));
      }
    };
    
    xhr.onerror = function() {
      console.error('XHR error:', xhr);
      if (xhr.status === 0) {
        reject(new Error('CORS error: Access blocked by CORS policy'));
      } else {
        reject(new Error('Network error during upload'));
      }
    };
    
    xhr.upload.onprogress = function(e) {
      if (e.lengthComputable) {
        const percentComplete = Math.round((e.loaded / e.total) * 100);
        console.log(\`Upload progress: \${percentComplete}%\`);
      }
    };
    
    xhr.send(file);
  });
};
```
