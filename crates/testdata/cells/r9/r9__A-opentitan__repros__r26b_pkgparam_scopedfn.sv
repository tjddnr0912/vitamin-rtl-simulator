package u; function automatic integer bits_for(integer n); return $clog2(n + 1); endfunction endpackage
package s;
  parameter int NumSt = 21;
  parameter int W = u::bits_for(NumSt);
endpackage
module t;
  initial begin #1 $display("A W=%0d", s::W); $finish; end
endmodule
