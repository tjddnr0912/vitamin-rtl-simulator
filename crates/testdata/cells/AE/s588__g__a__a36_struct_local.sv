module top;
  typedef struct packed { logic [1:0] a; logic [1:0] b; } s_t;
  function automatic logic [3:0] f(input int a);
    s_t s;
    f = s;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
