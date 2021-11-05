console.log("Hello, from JS");

const detailsSection = document.getElementById('details');

getDetails = () => {
    fetch('/info')
        .then(response => response.json())
        .then(data => {
            let html = '';
            detailsSection.textContent = '';
            
            data.forEach(d => {
                let section = document.createElement('p');
                section.innerText = d;
                detailsSection.appendChild(section);

            })
        })
        .catch(error => {
            console.log(error)
        });
}