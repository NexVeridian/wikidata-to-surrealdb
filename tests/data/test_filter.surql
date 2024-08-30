let $delete = select claims, id from Entity
where claims.claims[where id = Property:1113].value.Thing == [];

let $entity = return (select id from $delete).id;
let $claims = return (select claims from $delete).claims;

delete $claims;
delete $entity;
