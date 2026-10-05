module top;
  logic a, b; logic [1:0] m, y;
  function logic [1:0] f(input logic [1:0] v);
    f = 0;
    unique case (v) 2'b01: f = 1; 2'b10: f = 2; endcase
  endfunction
  always_comb m = {a, b};
  assign y = f(m);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t m=%b y=%b", $time, m, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
