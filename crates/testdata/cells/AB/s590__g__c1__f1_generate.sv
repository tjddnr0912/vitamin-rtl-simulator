module top;
  logic [1:0] a; logic [1:0] y;
  function logic f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    unique case (x) 1'b0: f = 1; 1'b1: f = 0; endcase
  endfunction
  for (genvar i = 0; i < 2; i++) begin : g
    assign y[i] = f(a[i]);
  end
  initial begin
    a = 2'b01;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
