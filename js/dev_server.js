
checkServer = (url) => {
     fetch(url)
	.then(response => response.json())
	.then(data => if data.reload() === true { reload() })
}

reload = () => {
    window.location.reload();
}

