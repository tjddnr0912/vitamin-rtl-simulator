package pa; localparam P = 3; localparam N = 5; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pk; import pa::*; import pb::P; localparam [64:0] Q = P + 65'd1; localparam [64:0] R = P; localparam int M = N; endpackage
module top; initial #1 $display("@ Q=%0d R=%0d M=%0d", pk::Q, pk::R, pk::M); endmodule
