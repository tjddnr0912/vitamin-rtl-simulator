module child #(parameter logic [3:0] P = fx(2)) ();
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    case (a) 1: t = 4'd5; endcase
    fx = t;
  endfunction
  initial begin #1 $display("%m P=%b", P); end
endmodule
module top;
  child u[1:0] ();
  initial #100 $finish;
endmodule
