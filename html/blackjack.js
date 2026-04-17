const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, output: process.stdout });        

function askQuestion(phrase) {
    return new Promise(resolve => rl.question(phrase, resolve));
}

class Table {
    constructor(id) {
        this.dealer = new Dealer;
        this.players = [];
        this.deck = new Deck;
        this.id = id;
    }

    addPlayer(name) {

        const newPlayer = new Player(name);
        this.players.push(newPlayer);
    }

    deal() {

        if (this.players.length > 0 && this.deck.length > 0) {
            let pickedCard = this.deck.cards.pop();
            this.players[0].hand.push(pickedCard);
            this.players[0].score = 11;
        }
    }

    getState() {
        return {
            tableId: this.id,
            RemainingCards: this.deck.length,
            players: this.players
        };
    }

    hit(player) {

        if (!player.canHit) {
            return ;
        }

        let card = this.deck.cards.pop();
        player.cards.push(card);
        player.sum += getValue(card)
        player.aceCount += checkAce(card);

        player.reducedSum = player.reduceAceScore();
        player.reduceAce();

        if (player.reducedSum >= 21) {
            player.canHit = false;
        }
        if (player.sum > 21 && player.reducedSum > 0 && player.reducedSum < player.sum) {
            player.sum = player.reducedSum;
        }
    }


    initGame() {

        this.deck.shuffleDeck();

        let hidden = this.deck.cards.pop();

        this.dealer.hiddenSum = getValue(hidden) // jsp si getValue opti
        this.dealer.aceCount += checkAce(hidden);        
        this.dealer.hidden = hidden;

        //console.log(hidden);
        //console.log(this.dealer.sum);
    
        let card = this.deck.cards.pop();

        this.dealer.sum += getValue(card)
        this.dealer.aceCount += checkAce(card);
        this.dealer.cards.push(card);

        for (let i = 0; i < this.players.length; i++) {
            
            for (let j = 0; j < 2; j++) {

                let card = this.deck.cards.pop();
                this.players[i].sum += getValue(card)
                this.players[i].aceCount += checkAce(card);
                this.players[i].cards.push(card);

                if (this.players[i].sum == 21) {

                    this.players[i].canHit = false;
                    this.players[i].blackjack = true;
                }
            }
        }
        //console.log(this.players);
    }

    endGame() {

        this.dealer.sum += this.dealer.hiddenSum;
        this.dealer.cards.push(this.dealer.hidden);

        console.log("Dealer has a score of : " + this.dealer.sum);
        console.log("Dealer cards : " + this.dealer.cards + "\n");

        while (this.dealer.sum < 17) {
            let card = this.deck.cards.pop();

            this.dealer.sum += getValue(card)
            this.dealer.aceCount += checkAce(card);
            this.dealer.cards.push(card);

            this.dealer.reduceAce();
            console.log("Dealer hitting !");
            console.log("Dealer has a score of : " + this.dealer.sum);
            console.log("Dealer cards : " + this.dealer.cards + "\n");
        }

        for (let i = 0; i < this.players.length; i++) {

            let message = "";

            if (this.players[i].sum > 21) {
                message = "You lose";
            }
            else if (this.dealer.sum > 21) {
                message = "You win";
            }
            else if (this.players[i].sum == this.dealer.sum) {
                message = "Push";
            }
            else if (this.players[i].sum > this.dealer.sum) {
                message = "You win";
            }
            else if (this.players[i].sum < this.dealer.sum) {
                message = "You loose";
            }
            console.log("Player : " + this.players[i].id +  " | " + message);
            console.log(this.players[i].id + " score : " + this.players[i].sum + ", dealer score : " + this.dealer.sum + "\n");
        }
    }

    async playGame() {

        for (let i = 0; i < this.players.length; i++) {

            if (this.players[i].canHit)
                await this.askHitStay(this.players[i]);
            else
                break;
        }

        this.endGame();
        rl.close();
    }

    async askHitStay(player) {

        console.log("\n[----- " + "Dealer" + " -----]\n");

        console.log(`Dealer cards : \x1b[32m[${this.dealer.cards}]\x1b[0m`);
        console.log("Dealer score : " + this.dealer.sum);

        console.log("\n[----- " + player.id + " -----]\n");
        
        while (player.canHit) {
            console.log(`Your cards : \x1b[32m[${player.cards}]\x1b[0m`);
            console.log("Your score is : " + player.sum);
            if (player.aceCount > 0) {
                console.log("Your minimal score : " + player.reduceAceScore());
            }

            if (player.sum > 21) {
                console.log("You burned ! End of your turn\n");
                player.canHit = false;
                break;
            }

            let action = await askQuestion('Action : (h)it ou (s)tay ? ');
            
            if (action === 'h') {
                console.log("Hitting a card.\n");
                this.hit(player);
                if (player.sum > 21) {
                    console.log(`Your cards : \x1b[32m[${player.cards}]\x1b[0m`);
                    console.log("Your score is : " + player.sum);
                    console.log("\x1b[31mYou burned ! End of your turn\x1b[0m\n");
                    break;
                } 
                else if (player.sum === 21) {
                    console.log(`Your cards : \x1b[32m[${player.cards}]\x1b[0m`);
                    console.log("\x1b[33m21 ! End of your turn\x1b[0m\n");
                    break;
                }
            } 
            else if (action === 's') {
                console.log("Staying, end of your turn\n");
                player.canHit = false;
                break;
            } 
            else {
                console.log("Invalid input\n");
            }
        }
    }
}

class Dealer {
    constructor() {
        this.sum = 0;
        this.hiddenSum = 0;
        this.aceCount = 0;
        this.cards = [];
    }

    reduceAce() {
        
        while (this.sum > 21 && this.aceCount > 0) {
            this.sum -= 10;
            this.aceCount -= 1;
        }

        return this.sum;
    }

}

class Player {
    constructor(name) {
        this.id = name;
        this.sum = 0;
        this.reducedSum = 0;
        this.score = 0;
        this.aceCount = 0;
        this.canHit = true;
        this.cards = [];
        this.blackjack = false;
    }

    reduceAce() {
        
        while (this.sum > 21 && this.aceCount > 0) {
            this.sum -= 10;
            this.aceCount -= 1;
        }

        return this.sum;
    }

    reduceAceScore() {
        
        let tmp_score = this.sum;
        let tmp_ace = this.aceCount;

        if (tmp_ace > 0)
            tmp_score -= 10;

        return tmp_score; // A CHECKER SELON REGLES
    }
}

class Deck {
    constructor() {    
        this.cards = buildDeck(); // opti construction deck
    }

    shuffleDeck() {

        for (let i = 0; i < this.cards.length; i++) {
            let j = Math.floor(Math.random() * this.cards.length);
            let temp = this.cards[i];
            this.cards[i] = this.cards[j];
            this.cards[j] = temp;
        }
    }
}

////////////////////UTILS///////////////////////////////

function getValue(card) {
    let data = card.split("-");
    let value = data[0];

    if (isNaN(value)) {
        if (value == "A") {
            return (11);
        }

        return (10);
    }

    return parseInt(value);
}

function buildDeck() {
        
    let values = ["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];
    let types = ["C", "D", "H", "S"];
    let deck = [];

    for (let i = 0; i < types.length; i++) {
        for (let j = 0; j < values.length; j++) {
            deck.push(values[j] + "-" + types[i]);
        }
    }
    return (deck);
}

function checkAce(card) {
    if (card[0] == "A") {
        return (1);
    }
    return (0);
}

//////////////////////////////////////////////////////////////

//TEST //

console.log("----- TRANSCENDANCE BLACKJACK -----");

const myTable = new Table();

myTable.addPlayer("Foo");
myTable.addPlayer("Bar");

// console.log("\n[Initial State] :");
// console.log(JSON.stringify(myTable.getState(), null, 2));

// console.log("\n--- Dealing cards... ---");
// myTable.deal();

// console.log("\n[State after dealing the cards] :");
// console.log(JSON.stringify(myTable.getState(), null, 2));

myTable.initGame();
myTable.playGame();
