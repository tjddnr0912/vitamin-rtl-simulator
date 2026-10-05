module top;
  typedef union tagged { void Inv; int Val; } U;
  U u = tagged Val 5;
  logic [1:0] r = 0;
  logic e = 0;
  initial begin
    #1 unique if (u matches tagged Inv) r = 1; else if (u matches tagged Val .x &&& x == 7) r = 2;
    #1 unique if (e) r = 1; else if (u matches tagged Val .x &&& x == 5) r = 2;
    #1 $finish;
  end
endmodule
