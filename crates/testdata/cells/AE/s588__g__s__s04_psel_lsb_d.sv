module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    case (a) 1: t = 4'd5; endcase
    fx = t;
  endfunction
  localparam logic [7:0] V = 8'hA5;
  initial begin #2 $display("r=%b", V[3:fx(2)]); $finish; end
  initial #100 $finish;
endmodule
