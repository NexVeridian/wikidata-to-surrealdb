# Get number of episodes
```
let $number_of_episodes = (select claims.claims[where id = Property:1113][0].value.ClaimValueData.Quantity.amount as number_of_episodes from Entity where label = "Black Clover, season 1")[0].number_of_episodes;

return $number_of_episodes;

update Entity SET number_of_episodes=$number_of_episodes where label = "Black Clover, season 1";
```

# Get Parts
```
let $parts = (select claims.claims[where id = Property:527].value.Thing as parts from Entity where label = "Black Clover")[0].parts;

return $parts;
```
