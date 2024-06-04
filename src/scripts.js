document.addEventListener('DOMContentLoaded', () => {
  const variantQueryCheckbox = document.getElementById('variantQueryCheckbox');
  const variantQueryInput = document.getElementById('variantQuery');
  const variantAskButton = document.getElementById('variantAskButton');
  const variantSuggestionsList = document.getElementById('variantSuggestionsList');
  const variantSuggestionsInput = document.getElementById('variantSuggestionsInput');

  const sampleQueryCheckbox = document.getElementById('sampleQueryCheckbox');
  const sampleQueryInput = document.getElementById('sampleQuery');
  const sampleAskButton = document.getElementById('sampleAskButton');
  const sampleSuggestionsList = document.getElementById('sampleSuggestionsList');
  const sampleSuggestionsInput = document.getElementById('sampleSuggestionsInput');

  const submitQueryButton = document.getElementById('submitQueryButton');
  const submitFilterButton = document.getElementById('submitFilterButton');
  const resultMessage = document.getElementById('resultMessage');

  const file = document.getElementById('file');

  variantQueryCheckbox.addEventListener('change', () => {
    variantQueryInput.disabled = !variantQueryCheckbox.checked;
  });

  sampleQueryCheckbox.addEventListener('change', () => {
    sampleQueryInput.disabled = !sampleQueryCheckbox.checked;
  });

  variantAskButton.addEventListener('click', () => {
    displaySuggestions(variantSuggestionsList, 'Variant', variantQueryInput, variantSuggestionsInput.value);
  });

  sampleAskButton.addEventListener('click', () => {
    displaySuggestions(sampleSuggestionsList, 'Sample', sampleQueryInput, sampleSuggestionsInput.value);
  });

  submitQueryButton.addEventListener('click', () => {
    let queryText = "";
    const vformatStr = 'CHROM + " " + POS';
    const sformatStr = 'IID + " " + SEX';
    if (sampleQueryCheckbox.checked && sampleQueryInput.value) {
      // if the user formats the string then we assume they know what they're doing and don't add any quotes etc. 
      if (sampleQueryInput.value.includes('-f')) {
        queryText += `-s -i ${sampleQueryInput.value}`;
      } else {
        queryText += `-s -i '${sampleQueryInput.value}' -f '${sformatStr}'`;
      }
    } 
    if (variantQueryCheckbox.checked && variantQueryInput.value) {
      if (queryText) {
        queryText += ` && -i `;
      } else {
        queryText += `-i `;
      }
      if (variantQueryInput.value.includes('-f')) {
        queryText += `${variantQueryInput.value}`;
      } else {
        queryText += `'${variantQueryInput.value}' -f '${vformatStr}'`;
      }
    }
    const querystr = `pgen-rs query ${queryText} ${file.value}`;
    console.log(querystr);
    submitQuery(querystr).then(success => {
      resultMessage.textContent = success ? 'Success! Query submitted.' : 'Error: Something went wrong.';
      resultMessage.style.color = success ? 'green' : 'red';
    });
  });

  submitFilterButton.addEventListener('click', () => {
    let queryText = "";
    if (sampleQueryCheckbox.checked && sampleQueryInput.value) {
      // if the user formats the string then we assume they know what they're doing and don't add any quotes etc. 
      if (sampleQueryInput.value.startsWith("'") && sampleQueryInput.value.endsWith("'") ) {
        queryText += `--include-sam ${sampleQueryInput.value} `;
      } else if (sampleQueryInput.value.startsWith("'") && !sampleQueryInput.value.endsWith("'") ) {
        queryText += `--include-sam ${sampleQueryInput.value}' `;
      } else if (!sampleQueryInput.value.startsWith("'") && sampleQueryInput.value.endsWith("'") ) {
        queryText += `--include-sam '${sampleQueryInput.value} `;
      } else {
        queryText += `--include-sam '${sampleQueryInput.value}' `;
      }
    } 
    if (variantQueryCheckbox.checked && variantQueryInput.value) {
      if (variantQueryInput.value.startsWith("'") && variantQueryInput.value.endsWith("'") ) {
        queryText += `--include-var ${variantQueryInput.value}`;
      } else if (variantQueryInput.value.startsWith("'") && !variantQueryInput.value.endsWith("'") ) {
        queryText += `--include-var ${variantQueryInput.value}'`;
      } else if (!variantQueryInput.value.startsWith("'") && variantQueryInput.value.endsWith("'") ) {
        queryText += `--include-var '${variantQueryInput.value}`;
      } else {
        queryText += `--include-var '${variantQueryInput.value}'`;
      }
    }

    const queryStr = `pgen-rs filter ${file.value} ${queryText}`
    console.log(queryStr);
    submitQuery(queryStr).then(success => {
      resultMessage.textContent = success ? 'Success! Filter(s) submitted.' : 'Error: Something went wrong.';
      resultMessage.style.color = success ? 'green' : 'red';
    });
  });

  async function displaySuggestions(listElement, queryType, queryInput, prompt) {
    const response = await fetch('/fetch_ai_response', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ pfile_prefix: file.value, prompt: prompt, query_type: queryType})
    });
    const responseBody = await response.text(); // Extract the response body
    const suggestions = responseBody.split('\n');
    listElement.innerHTML = '';
    suggestions.forEach(suggestion => {
      const listItem = document.createElement('li');
      listItem.textContent = suggestion;
      listItem.addEventListener('click', () => {
        queryInput.value = suggestion;
      });
      listElement.appendChild(listItem);
    });
  }

  async function submitQuery(query) {
    const response = await fetch('/submit_query', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({"query": query})
    });
    const responseBody = await response.text(); // Extract the response body
    console.log(responseBody);
    const success = responseBody.includes('Success');
    return success
  }
});
