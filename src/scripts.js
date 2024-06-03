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

  const submitButton = document.getElementById('submitButton');
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

  submitButton.addEventListener('click', () => {
    let queryText = "";
    const formatStr = 'CHROM + " " + POS';
    if (sampleQueryCheckbox.checked) {
      if (sampleQueryInput.value != "") {
        // if the user formats the string then we assume they know what they're doing and don't add any quotes etc. 
        if (sampleQueryInput.value.includes('-f')) {
          queryText += `-s ${sampleQueryInput.value}`
        } else {
          queryText += `-s '${sampleQueryInput.value}' -f '${formatStr}'`
        }
      }
    } else if (variantQueryCheckbox.checked) {
      if (variantQueryInput.value.includes('-f')) {
        queryText += `${variantQueryInput.value}`
      } else {
        queryText += `'${variantQueryInput.value}' -f '${formatStr}'`
      }
    }
    
    const querystr = `pgen-rs query --include ${queryText} ${file.value}`;
    submitQuery(querystr).then(success => {
      resultMessage.textContent = success ? 'Success! Queries submitted.' : 'Error: Something went wrong.';
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
