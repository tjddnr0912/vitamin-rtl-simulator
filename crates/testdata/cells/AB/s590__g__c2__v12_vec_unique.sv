module top;
  logic [1:0] a; logic [1:0] y;
  function logic [1:0] f(input logic [1:0] v);
    f = 0;
    unique case (v) 2'b01: f = 1; 2'b10: f = 2; endcase
  endfunction
  assign y = f(a);
  initial begin
    a = 2'b01;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
