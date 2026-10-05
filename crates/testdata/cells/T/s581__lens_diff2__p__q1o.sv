package pa; localparam P = 3; localparam N = 5; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  if (1) begin : b
    import pa::*;
    import pb::P;
    localparam [64:0] Q = P + 65'd1;
    wire [64:0] w = P;
    initial #1 $display("@ P=%0d Q=%0d w=%0d N=%0d", P, Q, w, N);
  end
endmodule
