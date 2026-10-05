module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic clk = 0; int v = 7;
  covergroup cg_t @(posedge clk);
    coverpoint v { bins b = {f(2)}; }
  endgroup
  cg_t cg = new;
  initial begin #1 clk = 1; #1 $display("cov=%0.1f", cg.get_coverage()); $finish; end
endmodule
