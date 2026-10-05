module top;
  logic a, b; logic [1:0] y;
  function automatic logic [1:0] f(input logic x, input logic z);
    logic [1:0] r;
    r = 0;
    unique case ({x, z}) 2'b01: r = 1; 2'b10: r = 2; endcase
    return r;
  endfunction
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
