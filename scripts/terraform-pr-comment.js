const fs = require('fs');

module.exports = async ({github, context}) => {
  const planOutput = fs.readFileSync('terraform/plan_output.txt', 'utf8');
  const truncatedPlan = planOutput.length > 65000 
    ? planOutput.substring(0, 65000) + '\n\n... (truncated)'
    : planOutput;
  
  const comment = `## Terraform Plan Results
  
  <details>
  <summary>Click to expand plan details</summary>
  
  \`\`\`terraform
  ${truncatedPlan}
  \`\`\`
  
  </details>
  
  **Note:** This plan will be automatically applied when merged to main.`;
  
  await github.rest.issues.createComment({
    issue_number: context.issue.number,
    owner: context.repo.owner,
    repo: context.repo.repo,
    body: comment
  });
};