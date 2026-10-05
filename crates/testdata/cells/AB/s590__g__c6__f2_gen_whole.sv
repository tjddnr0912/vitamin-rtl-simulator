module top;
  logic a, b; logic [1:0] y;
  function logic f(input logic x, input logic z);
    f = 0;
    unique case ({x, z}) 2'b01: f = 1; 2'b10: f = 0; endcase
  endfunction
  for (genvar i = 0; i < 2; i++) begin : g
    assign y[i] = f(a, b);
  end
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
