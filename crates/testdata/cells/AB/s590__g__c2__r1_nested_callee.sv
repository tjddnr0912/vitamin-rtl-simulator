module top;
  logic a, b; logic [1:0] y;
  function logic [1:0] h(input logic [1:0] v);
    h = 0;
    unique case (v) 2'b01: h = 1; 2'b10: h = 2; endcase
  endfunction
  function logic [1:0] f(input logic x, input logic z);
    return h({x, z});
  endfunction
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
