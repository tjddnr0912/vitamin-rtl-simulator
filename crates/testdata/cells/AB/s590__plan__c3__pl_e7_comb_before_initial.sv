module top;
  logic a, b; logic [1:0] y; logic [1:0] c;
  function logic [1:0] f(input logic x, input logic z);
    f = 0;
    unique case ({x, z}) 2'b01: f = 1; 2'b10: f = 2; endcase
  endfunction
  always_comb begin
    c = y;
    unique case (y) 2'd1: ; 2'd2: ; endcase
  end
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b c=%b", $time, y, c);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
