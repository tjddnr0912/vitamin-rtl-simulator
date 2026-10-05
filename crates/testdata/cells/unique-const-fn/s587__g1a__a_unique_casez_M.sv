module top;
  function automatic int f(input int a);
    f = 7;
    unique casez (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  localparam int P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
