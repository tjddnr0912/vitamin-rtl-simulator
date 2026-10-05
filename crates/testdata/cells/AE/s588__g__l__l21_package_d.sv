package pk;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    case (a) 1: t = 4'd5; endcase
    fx = t;
  endfunction
  localparam logic [3:0] P = fx(2);
endpackage
module top;
  initial begin #1 $display("P=%b", pk::P); $finish; end
  initial #100 $finish;
endmodule
