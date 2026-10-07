package u; function automatic integer bits_for(integer n); return $clog2(n + 1); endfunction endpackage
package s;
  import u::bits_for;
  parameter int NumSt = 21;
  parameter int W = bits_for(NumSt);
  parameter int R = 32 / W;
endpackage
module t;
  initial begin #1 $display("A W=%0d R=%0d", s::W, s::R); $finish; end
endmodule
